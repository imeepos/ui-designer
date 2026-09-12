#!/usr/bin/env bash
# 发布产物冒烟脚本：给定 tag，对已发布的 GitHub Release 资产做安装前置健康检查。
#
# 背景：产物无开发者签名/未公证（release.yml 未配 TAURI_SIGNING_*/签名证书；实测
# v0.2.0 为 adhoc+linker-signed），macOS Gatekeeper 会拦浏览器下载的安装包。本脚本
# 在用户人工安装前，先把「产物本身是否健康」验证清楚，让用户报障能快速二分为
# 「产物问题」（本脚本 FAIL）与「环境问题」（本脚本 PASS 但用户仍报障）。
#
# 下载通道策略（v0.2.0 冒烟实测沉淀：github.com 直连被网络环境阻断时脚本仍可用）：
#   通道1  github.com 的 browser_download_url —— 用户浏览器同款入口，先探测可达性
#   通道2  api.github.com 资产直取（Accept: application/octet-stream → 302 →
#          objects.githubusercontent.com CDN）——不依赖 github.com 主站
#   若设置 SMOKE_PROXY=http://host:port，两条通道失败后各经代理重试一次。
#   刻意不自动读 macOS 系统代理：实测系统代理可能对 github.com TLS 复位而
#   api.github.com 直连正常，自动改道反而全挂；代理只认显式 SMOKE_PROXY。
#
# 检查链（全部真实退出码判定，任何 FAIL 使脚本整体退出码非 0）：
#   下载 dmg + SHA256SUMS.txt + 裸 CLI → SHA256 按 basename 对账（CI 的 SUMS 路径带
#   artifacts 子目录前缀，不能直接 shasum -c）→ hdiutil 只读挂载 dmg → 校验 .app 结构
#   （Contents/MacOS 主二进制存在且可执行、Info.plist CFBundleShortVersionString=tag 版本）
#   → sidecar CLI 二进制存在且 --version 输出正确版本 → codesign 状态如实记录
#   （实测 adhoc/linker-signed，无有效签名属预期，非失败项）→ 卸载并清理临时挂载
#   与 /tmp 工作目录。
#
# 用法：
#   scripts/release/smoke-release.sh v0.2.0
# 环境变量：
#   RELEASE_REPO   仓库，默认 imeepos/ui-designer
#   SMOKE_PROXY    可选代理（http://host:port），下载重试用
#   KEEP_WORK=1    保留 /tmp 工作目录（调试用），仍会卸载 dmg
#
# 仅支持 macOS（依赖 hdiutil/codesign/PlistBuddy）；产物按 host 架构选择 aarch64/x86_64。
# 下载只落 /tmp，不进仓库；不安装 app、不改动任何远端资源。

set -euo pipefail

REPO="${RELEASE_REPO:-imeepos/ui-designer}"
TAG="${1:-}"
KEEP_WORK="${KEEP_WORK:-0}"
PROXY="${SMOKE_PROXY:-}"

PASS_COUNT=0
FAIL_COUNT=0
WORK=""
MP=""
MOUNTED=0
GITHUB_COM_OK=0

# ---------- 输出与判定基元 ----------

say() { echo "smoke: $*"; }

step_result() { # $1=步骤名 $2=退出码；PASS/FAIL 唯一判定来源就是真实退出码
  if [[ "$2" -eq 0 ]]; then
    say "[$1] exit=0 PASS"
    PASS_COUNT=$((PASS_COUNT + 1))
  else
    say "[$1] exit=$2 FAIL" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
  fi
}

# 在 if 条件里跑命令：set -e 不杀脚本，退出码如实带回
run_step() { # $1=步骤名，其余=命令
  local desc="$1"
  shift
  local out rc
  if out=$("$@" 2>&1); then rc=0; else rc=$?; fi
  if [[ -n "$out" ]]; then
    printf '%s\n' "$out" | sed 's/^/    | /'
  fi
  step_result "$desc" "$rc"
  return "$rc"
}

fatal() { # 前置条件失败，无法继续后续检查
  say "FATAL $*（无法继续，已完成检查 $PASS_COUNT 通过 / $FAIL_COUNT 失败）" >&2
  exit 2
}

# ---------- 参数与平台前置 ----------

if [[ -z "$TAG" ]]; then
  echo "用法: $0 <tag>   例: $0 v0.2.0" >&2
  exit 64
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "smoke: FAIL 本脚本依赖 hdiutil/codesign，仅支持 macOS（当前 $(uname -s)）" >&2
  exit 2
fi

case "$(uname -m)" in
  arm64) MAC_ARCH="aarch64" ;;
  x86_64) MAC_ARCH="x86_64" ;;
  *)
    echo "smoke: FAIL 无法识别的 host 架构：$(uname -m)" >&2
    exit 2
    ;;
esac

VERSION="${TAG#v}"
DMG_NAME="Rudder_${VERSION}_${MAC_ARCH}.dmg"
CLI_NAME="rudder-${MAC_ARCH}-apple-darwin"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"

WORK="$(mktemp -d "/tmp/rudder-smoke-${TAG}-XXXXXX")"

cleanup() {
  local rc=$?
  if [[ "$MOUNTED" -eq 1 ]]; then
    if hdiutil detach "$MP" -quiet; then
      say "清理 [hdiutil detach] exit=0"
    else
      say "清理 [hdiutil detach] FAIL（请手动: hdiutil detach '$MP'）" >&2
    fi
    MOUNTED=0
  fi
  if [[ "$KEEP_WORK" == "1" ]]; then
    say "KEEP_WORK=1，保留工作目录 $WORK"
  else
    rm -rf "$WORK"
  fi
  exit "$rc"
}
trap cleanup EXIT
# SIGTERM/SIGINT（超时被杀、Ctrl-C）也转入 EXIT 清理：卸载 dmg + 删 /tmp 工作目录
trap 'exit 143' TERM
trap 'exit 130' INT

# ---------- 下载基元：直连 → （可选）代理；短连接超时 + 停滞熔断防挂死 ----------
# 实测 v0.2.0 冒烟：本机到 github 链路极不稳定（同一天内直连时通时断、TCP 建连后
# 可无限停滞），--connect-timeout 只管建连，传输停滞靠 --speed-time/--speed-limit 熔断
CURL_BASE=(-fsSL --http1.1 --connect-timeout 15 --max-time 240 \
  --speed-limit 1024 --speed-time 30 --retry 3 --retry-all-errors)

try_curl() { # $1=输出文件 $2=url，其余=追加 curl 参数
  local out_file="$1" url="$2"
  shift 2
  curl "${CURL_BASE[@]}" "$@" -o "$out_file" "$url"
}

download() { # $1=输出文件 $2=url：先直连，失败且配了 SMOKE_PROXY 则走代理重试
  local out_file="$1" url="$2" rc
  if try_curl "$out_file" "$url"; then
    return 0
  fi
  rc=$?
  if [[ -n "$PROXY" ]]; then
    say "  直连失败（curl exit=${rc}），经 SMOKE_PROXY=${PROXY} 重试…"
    if try_curl "$out_file" "$url" --proxy "$PROXY"; then
      say "  经代理下载成功"
      return 0
    fi
    rc=$?
  fi
  return "$rc"
}

probe_github_com() { # 通道1 可达性一次性探测，决定后续资产是否跳过浏览器入口
  local rc=0
  curl -fsI --http1.1 --connect-timeout 8 --max-time 12 https://github.com/ >/dev/null 2>&1 || rc=$?
  if [[ "$rc" -eq 0 ]]; then
    GITHUB_COM_OK=1
    say "前置探测: github.com 直连可达（通道1 启用）"
  else
    say "前置探测: github.com 直连不可达（curl exit=${rc}）——资产改走 api.github.com 通道"
  fi
}

asset_id() { # $1=资产名 → GitHub asset id（api.github.com 元数据解析，文件缓存可跨子 shell）
  local name="$1"
  local cache="${WORK}/release-${TAG}.json"
  if [[ ! -s "$cache" ]]; then
    download "$cache" "https://api.github.com/repos/${REPO}/releases/tags/${TAG}" \
      || { say "  api.github.com 拉取 Release 元数据失败"; return 1; }
  fi
  # API pretty JSON 缩进实测（v0.2.0）：顶层字段 2 空格、author 4 空格、
  # assets[] 资产对象字段 6 空格（其 uploader 8 空格）。按 6 空格锚定资产 id，
  # 与资产名精确比对（剥离「name": "…",」装饰后全等），避开 author/顶层同名干扰
  awk -v want="$name" '
    /^      "id": [0-9]+,/ { id = $2; gsub(",", "", id) }
    /^      "name": "/ {
      line = $0
      gsub(/^      "name": "/, "", line)
      gsub(/",?$/, "", line)
      if (line == want) { print id; exit }
    }
  ' "$cache"
}

fetch_asset() { # $1=资产名 → 双通道下载到 $WORK
  local name="$1"
  say "下载资产 ${name}"
  if [[ "$GITHUB_COM_OK" -eq 1 ]]; then
    say "  通道1: github.com browser_download_url（浏览器同款入口）"
    if download "${WORK}/${name}" "${BASE_URL}/${name}"; then
      say "  下载成功（$(stat -f%z "${WORK}/${name}") 字节）"
      return 0
    fi
  else
    say "  通道1跳过: github.com 不可达（见前置探测）"
  fi
  say "  通道2: api.github.com 资产直取（octet-stream → objects CDN）"
  local id rc
  id="$(asset_id "$name")" || { say "  资产 id 解析失败"; return 1; }
  if [[ -z "$id" ]]; then
    say "  Release ${TAG} 中不存在资产 ${name}"
    return 1
  fi
  if download "${WORK}/${name}" "https://api.github.com/repos/${REPO}/releases/assets/${id}" \
    -H "Accept: application/octet-stream"; then
    say "  下载成功（$(stat -f%z "${WORK}/${name}") 字节，asset id=${id}）"
    return 0
  fi
  rc=$?
  say "  全部通道失败（最后 exit=${rc}）"
  return "$rc"
}

# ---------- 校验基元 ----------

sha_of() { # $1=文件路径 → sha256 十六进制
  shasum -a 256 "$1" | awk '{print $1}'
}

verify_sha() { # $1=资产名：与 SHA256SUMS.txt 按 basename 对账
  local name="$1"
  local sums="${WORK}/SHA256SUMS.txt"
  local actual expected
  actual="$(sha_of "${WORK}/${name}")"
  # SUMS 行格式：<hash>  ./<artifact目录>/.../<name>（CI 在 artifacts/ 内生成，路径带前缀），
  # 故按「路径以 /<name> 结尾」对账，而不是整行比对。
  # index 返回「/资产名」的 1 基起点 = length-m（m 为资产名长度），即 basename 前那个斜杠
  expected="$(awk -v n="$name" 'BEGIN { m = length(n) }
    index($2, "/" n) == length($2) - m { print $1; exit }' "$sums")"
  say "  SHA256 本地实值   = ${actual}"
  if [[ -z "$expected" ]]; then
    say "  SHA256SUMS.txt 中未找到 ${name} 条目"
    return 1
  fi
  say "  SHA256 清单实值   = ${expected}"
  [[ "$actual" == "$expected" ]]
}

check_app_found() { # 挂载点内应恰有一个 .app
  local found
  found="$(find "$MP" -maxdepth 2 -type d -name '*.app' | head -5)"
  if [[ -z "$found" ]]; then
    say "  挂载点内未发现 .app"
    return 1
  fi
  local count
  count="$(printf '%s\n' "$found" | wc -l | tr -d ' ')"
  say "  发现 .app（${count} 个）: ${found//$'\n'/, }"
  [[ "$count" -eq 1 ]]
}

# ---------- 开始冒烟 ----------

say "===== ${REPO} @ ${TAG}（host: $(uname -m), macOS $(sw_vers -productVersion 2>/dev/null || echo '?')）====="
say "工作目录: ${WORK}"
[[ -n "$PROXY" ]] && say "SMOKE_PROXY=${PROXY}（下载失败时经其重试）"

probe_github_com

run_step "下载 dmg（${DMG_NAME}）" fetch_asset "$DMG_NAME" \
  || fatal "dmg 下载失败（网络或资产缺失，见上方 curl 退出码）"
run_step "下载 SHA256SUMS.txt" fetch_asset "SHA256SUMS.txt" \
  || fatal "SHA256SUMS.txt 下载失败"
run_step "下载裸 CLI（${CLI_NAME}）" fetch_asset "$CLI_NAME" \
  || fatal "裸 CLI 下载失败"

run_step "SHA256 校验 ${DMG_NAME}" verify_sha "$DMG_NAME" || true
run_step "SHA256 校验 ${CLI_NAME}" verify_sha "$CLI_NAME" || true

# curl 下载不携带 quarantine 扩展属性（浏览器会带）——如实记录，解释本地能直接跑、
# 用户从浏览器装会被 Gatekeeper 拦的形态差异
if xattr -l "${WORK}/${DMG_NAME}" 2>/dev/null | grep -q "com.apple.quarantine"; then
  say "备注: 下载文件带 com.apple.quarantine（本机将由 Gatekeeper 首次校验）"
else
  say "备注: curl 下载文件不带 quarantine 属性（浏览器下载会带，Gatekeeper 形态见报告）"
fi

MP="${WORK}/mnt"
mkdir -p "$MP"
run_step "挂载 dmg（hdiutil attach -readonly）" hdiutil attach -readonly -nobrowse -mountpoint "$MP" "${WORK}/${DMG_NAME}" \
  || fatal "dmg 挂载失败（镜像损坏或下载不完整）"
MOUNTED=1

run_step ".app 结构：挂载点内恰有一个 .app" check_app_found \
  || fatal "挂载点内 .app 数量非 1，与任务书假设不符（NEEDS_CONTEXT）"

APP="$(find "$MP" -maxdepth 2 -type d -name '*.app' | head -1)"
PLIST="${APP}/Contents/Info.plist"

# Info.plist 关键键在父 shell 读取：run_step 捕获输出的命令替换是子 shell，
# 在其内部赋值的变量不会带回（v0.2.0 首跑实测教训，MAIN_BIN 因此丢失）
PLIST_SHORT_VER="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$PLIST" 2>/dev/null || true)"
PLIST_EXEC="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$PLIST" 2>/dev/null || true)"
PLIST_ID="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$PLIST" 2>/dev/null || true)"
MAIN_BIN="${APP}/Contents/MacOS/${PLIST_EXEC}"

check_plist_and_main_bin() {
  if [[ -z "$PLIST_EXEC" || -z "$PLIST_SHORT_VER" ]]; then
    say "  Info.plist 关键键（CFBundleExecutable/CFBundleShortVersionString）读取失败"
    return 1
  fi
  say "  CFBundleShortVersionString = ${PLIST_SHORT_VER}（期望 ${VERSION}）"
  say "  CFBundleExecutable         = ${PLIST_EXEC}"
  say "  主二进制                    = ${MAIN_BIN}"
  [[ -f "$MAIN_BIN" && -x "$MAIN_BIN" ]]
}

run_step ".app 结构：主二进制存在且可执行" check_plist_and_main_bin || true
run_step "版本一致：CFBundleShortVersionString=${VERSION}" test "$PLIST_SHORT_VER" = "$VERSION" || true
say "  bundle id: ${PLIST_ID:-(不可读)}"

check_sidecar() { # Tauri externalBin（binaries/rudder）落盘实测为 Contents/MacOS/rudder，
  # 不带 triple 后缀（v0.2.0 实盘发现，triple 后缀只在裸 CLI 资产名上）——故按
  # 「除主二进制外的文件」发现候选，≥1 个 --version 出正确版本即 PASS
  if [[ -z "$MAIN_BIN" || ! -e "$MAIN_BIN" ]]; then
    say "  主二进制未确认，无法排除同名干扰"
    return 1
  fi
  local candidates
  candidates="$(find "${APP}/Contents/MacOS" -maxdepth 1 -type f ! -path "$MAIN_BIN")"
  if [[ -z "$candidates" ]]; then
    say "  Contents/MacOS 下除主二进制外无其他文件（sidecar 缺失？）"
    ls -la "${APP}/Contents/MacOS" | sed 's/^/    | /'
    return 1
  fi
  local bin out found_ok=0
  while IFS= read -r bin; do
    [[ -z "$bin" ]] && continue
    say "  候选 sidecar: ${bin}（$(stat -f%z "$bin") 字节）"
    file "$bin" | sed 's/^/    | /'
    if out="$("$bin" --version 2>&1)"; then
      if [[ "$out" == *"$VERSION"* ]]; then
        say "  --version => ${out} PASS"
        found_ok=1
      else
        say "  --version => ${out}（版本号不含 ${VERSION}）"
      fi
    else
      say "  --version 执行失败：${out}（伴随文件备注，不判 FAIL）"
    fi
  done <<< "$candidates"
  [[ "$found_ok" -eq 1 ]]
}

run_step "sidecar CLI 存在且 --version=${VERSION}" check_sidecar || true

# codesign：任务书预期 unsigned；实测 v0.2.0 为 adhoc+linker-signed（ld 自动签，无开发者
# 身份、未公证，Gatekeeper 依旧拦截）——两种形态同属「无有效签名」家族，只记录不计 FAIL
say "--- codesign 状态（预期无有效签名，非失败项，如实记录）---"
codesign --verify --deep --strict "$APP" 2>&1 | sed 's/^/    | /' || true
codesign -dv "$APP" 2>&1 | sed 's/^/    | /' || true
spctl --assess --type execute "$APP" 2>&1 | sed 's/^/    | /' || true

# ---------- 裸 CLI：非 GUI 装法的前置验证 ----------

chmod +x "${WORK}/${CLI_NAME}"
say "--- 裸 CLI ${CLI_NAME} ---"
file "${WORK}/${CLI_NAME}" | sed 's/^/    | /'

check_bare_cli() {
  local out
  if ! out="$("${WORK}/${CLI_NAME}" --version 2>&1)"; then
    say "  --version 执行失败: ${out}"
    return 1
  fi
  say "  --version => ${out}（期望含 ${VERSION}）"
  [[ "$out" == *"$VERSION"* ]]
}

run_step "裸 CLI --version 退出码 0 且含 ${VERSION}" check_bare_cli || true

# ---------- 汇总 ----------

say "===== 冒烟汇总: ${PASS_COUNT} PASS / ${FAIL_COUNT} FAIL ====="
if [[ "$FAIL_COUNT" -gt 0 ]]; then
  say "FAIL 结论：产物或校验链存在问题，用户报障应先按产物问题排查"
  exit 1
fi
say "PASS 结论：产物健康，用户报障优先按环境问题排查（Gatekeeper/钥匙串/网络）"
exit 0
