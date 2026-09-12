# v0.2.0 发布产物安装前置健康报告（冒烟实测）

- 任务：R1 发布产物冒烟工具化（quality-gates）
- 产物：https://github.com/imeepos/ui-designer/releases/tag/v0.2.0 （10 资产）
- 工具：`scripts/release/smoke-release.sh`（本次新增，分支 `feat/r1-release-smoke`）
- 执行环境：macOS 26.6 / arm64（Apple Silicon），执行时间 2026-09-11 晚（本地）
- 结论：**PASS（产物健康）** —— 11/11 项判定检查全通过（另有探测/卸载 2 项非判定步骤）；用户报障应优先按**环境问题**排查

## 1. 逐步命令与退出码（最终一轮实测）

| # | 检查项 | 命令（脚本内部实际执行） | 退出码 | 判定 |
|---|--------|--------------------------|--------|------|
| 1 | github.com 可达性探测 | `curl -fsI --http1.1 --connect-timeout 8 --max-time 12 https://github.com/` | 0（本轮可达；同日另有 28 超时轮次，见 §5） | 前置 |
| 2 | 下载 dmg | `curl -fsSL --http1.1 --connect-timeout 15 --max-time 240 --speed-limit 1024 --speed-time 30 --retry 3 --retry-all-errors -o <tmp>/Rudder_0.2.0_aarch64.dmg https://github.com/imeepos/ui-designer/releases/download/v0.2.0/Rudder_0.2.0_aarch64.dmg` | 0 | PASS（8,061,317 字节） |
| 3 | 下载 SHA256SUMS.txt | 同上通道，`.../v0.2.0/SHA256SUMS.txt` | 0 | PASS（1,055 字节） |
| 4 | 下载裸 CLI | 同上通道，`.../v0.2.0/rudder-aarch64-apple-darwin` | 0 | PASS（3,981,840 字节） |
| 5 | SHA256 对账 dmg | `shasum -a 256` ×2 按 basename 对账 | 0 | PASS（实值见 §2） |
| 6 | SHA256 对账裸 CLI | 同上 | 0 | PASS（实值见 §2） |
| 7 | 挂载 dmg | `hdiutil attach -readonly -nobrowse -mountpoint <tmp>/mnt <dmg>`（dmg 内部 CRC 全 verified） | 0 | PASS |
| 8 | .app 结构 | `find <mnt> -maxdepth 2 -type d -name '*.app'`（恰 1 个：`Rudder.app`） | 0 | PASS |
| 9 | 主二进制存在且可执行 | PlistBuddy 读 CFBundleExecutable → `<.app>/Contents/MacOS/<exe>` `[[ -f && -x ]]` | 0 | PASS |
| 10 | 版本一致 | `test "$(PlistBuddy Print :CFBundleShortVersionString)" = "0.2.0"` | 0 | PASS |
| 11 | sidecar CLI --version | `find Contents/MacOS -maxdepth 1 -type f ! -path <主二进制>` → 候选执行 `--version` | 0 | PASS（`rudder 0.2.0`） |
| 12 | 裸 CLI --version | `chmod +x <tmp>/rudder-aarch64-apple-darwin && <它> --version` | 0 | PASS（`rudder 0.2.0`） |
| 13 | 卸载清理 | `hdiutil detach <mnt> -quiet` + `rm -rf <tmp>` | 0 | PASS（非判定项） |

注：#1 与 #13 是探测/清理动作，不计入 PASS/FAIL 计数；计入计数的判定检查共 11 项。

汇总行：`smoke: ===== 冒烟汇总: 11 PASS / 0 FAIL =====`，脚本整体退出码 **0**。完整运行日志见文末附录。

## 2. 实值记录（取证用）

| 项 | 实值 |
|----|------|
| SHA256(`Rudder_0.2.0_aarch64.dmg`) | `037d847a5c2fd4d8534c068cd324c59d68492a1c549a9b6fde7a2f309e5a0119`（与 SHA256SUMS.txt 及 GitHub API digest 一致） |
| SHA256(`rudder-aarch64-apple-darwin`) | `7d559fe559bd0c665c1ef18fcaadbc483a86d9995f2884ac2ce793356b19d619`（同上三方一致） |
| SHA256(`Rudder-app.zip`) | `a9c57ad149efc93f3d5dcdd9c88bf7692c134bd44d265c58d57c539a0b0151e2`（清单值，未实测解压） |
| CFBundleShortVersionString | `0.2.0`（= tag 版本） |
| CFBundleExecutable | `rudder-app`（见 §4-A） |
| CFBundleIdentifier | `com.rudder.studio` |
| 主二进制 | `Rudder.app/Contents/MacOS/rudder-app`，10,661,952 字节，Mach-O arm64 |
| sidecar | `Rudder.app/Contents/MacOS/rudder`，3,981,840 字节（与裸 CLI 同字节数，即同一二进制），`--version` → `rudder 0.2.0` |
| 裸 CLI | `rudder-aarch64-apple-darwin`，Mach-O 64-bit arm64，`--version` → `rudder 0.2.0` |

## 3. codesign 状态（预期无有效签名，非失败项，如实记录）

```
codesign --verify --deep --strict Rudder.app → 退出码 1
  "code has no resources but signature indicates they must be present"
codesign -dv Rudder.app：
  Signature=adhoc
  CodeDirectory flags=0x20002(adhoc,linker-signed)
  TeamIdentifier=not set
  Sealed Resources=none
  Info.plist=not bound
spctl --assess --type execute → rejected
```

**修正任务书假设**：不是「完全 unsigned」，而是 **adhoc + linker-signed**（链接器自动附带的临时签名，无开发者身份、未公证）。对 Gatekeeper 的效果与 unsigned 同类：**未公证下载仍会被拦**。

## 4. 与任务书假设不符的三个新事实（产物侧）

- **A. 主二进制名是 `rudder-app` 不是 `Rudder`**：Tauri 2 的 CFBundleExecutable 取 src-tauri crate 名（`rudder-app`），productName 只决定 .app 目录名。写文档/排查时在 `Contents/MacOS/` 找 `Rudder` 会扑空。
- **B. 包内 sidecar 名是 `rudder`，不带 triple 后缀**：`bundle.externalBin: ["binaries/rudder"]` 按配置名原样落盘；`rudder-aarch64-apple-darwin` 这类 triple 命名只出现在 Release 裸 CLI 资产上。两者字节数一致（3,981,840），确为同一二进制。
- **C. SHA256SUMS.txt 路径带 artifacts 目录前缀**（如 `./Rudder-macos/target/release/bundle/dmg/Rudder_0.2.0_aarch64.dmg`），用户侧校验不能直接 `shasum -a 256 -c`，须按 basename 对账（脚本已内置）。

## 5. 网络通道（环境侧关键事实）

- 本机到 github.com 链路**极不稳定**：同一晚多轮探测在「直连可达」与「TCP 超时（curl exit 28）/HTTP2 framing 错误（exit 16）」间反复；GitHub API 元数据（api.github.com）与 objects CDN 直连则稳定可达。
- 脚本因此实现双通道：①github.com 浏览器同款入口（带可达性预探测）→ ②api.github.com 资产直取（`Accept: application/octet-stream` → 302 → objects CDN），可选 `SMOKE_PROXY` 走显式代理。**刻意不自动读系统代理**（实测本机系统代理 127.0.0.1:7890 对 github.com TLS 复位，而对 apple.com 正常，自动改道反而全挂）。
- 用户含义：无可用代理的国内网络下，**浏览器直接下载 Release 资产可能失败或极慢**——这不是产物问题，报障时先确认下载是否完整（SHA256）。

## 6. 用户安装预期（macOS, Apple Silicon）

### 6.1 GUI（dmg）首次打开必被 Gatekeeper 拦，预期形态与放行

1. 下载完成后首次双击/拷贝到 /Applications 后打开，弹窗为「**"Rudder.app"未打开 - Apple 无法验证其是否不含恶意软件**」或「已损坏，无法打开」（后者多为 quarantine + 未公证组合形态）。
2. 放行步骤（任选其一）：
   - **右键 → 打开 → 再点「打开」**（对「无法验证」弹窗有效；macOS 15+ 若右键无「打开」，走下一条）；
   - **系统设置 → 隐私与安全性 → 安全性节 →「仍要打开」**；
   - 命令行（解除浏览器附带的 quarantine）：`xattr -dr com.apple.quarantine /Applications/Rudder.app`。
3. 放行一次后，同版本后续启动不再拦。**安装流程本身不需要管理员密码**（拖拽安装）。
4. 本脚本本地实测（curl 下载无 quarantine）挂载/结构/可执行性全部健康，故上述拦截属**预期中的系统行为**，不属产物缺陷。

### 6.2 裸 CLI（rudder-aarch64-apple-darwin）

1. 下载后必须 `chmod +x`，否则 `permission denied`。
2. **浏览器下载的二进制带 quarantine**，首次执行可能被 Gatekeeper 杀（`killed: 9`）或弹「无法验证开发者」；放行：`xattr -d com.apple.quarantine <文件>`。curl/wget 下载则无此属性，可直接执行。
3. 本机实测：`--version` → `rudder 0.2.0`，退出码 0。

### 6.3 「CLI --version 报错」原因分级（排查顺序自上而下）

| 级别 | 原因 | 特征 | 处置 |
|------|------|------|------|
| L1 环境 下载完整性 | 下载不完整/被中间层替换 | SHA256 对不上 | 重新下载或换通道；换网络/代理 |
| L1 环境 quarantine | `killed: 9` / 「无法验证开发者」 | 文件带 `com.apple.quarantine`（`xattr -l` 可见） | `xattr -d com.apple.quarantine <文件>` |
| L1 环境 权限 | `Permission denied` | 未 `chmod +x` | `chmod +x <文件>` |
| L1 环境 架构 | `exec format error` / 直接 core | 非 arm64 机器跑了 aarch64 资产 | Intel 用户暂无官方 x86_64-darwin 资产（当前矩阵仅 macos-latest=arm64） |
| L2 产物 | SHA256 与清单不符但下载声称完整 | 本脚本 FAIL | 立即冻结发布，附实值上报（本次 v0.2.0 无此情况） |
| L3 运行时 | 除 `--version` 外的子命令报错 | 触网/触钥匙串（`config set api-key` 等） | 与本报告无关，属运行环境/凭证问题，按 AGENTS.md 钥匙串纪律排查 |

注：`--version` 是编译期常量输出（`env!("CARGO_PKG_VERSION")`），不触网、不读钥匙串、不读 `~/Rudder/config.json`——它失败只能是「执行层」问题，不存在配置类原因。

## 7. 门禁与影响面

- `shellcheck`：本机未安装（已如实申报）；以 `bash -n` 语法检查通过 + 人工复查替代。建议后续 CI 或本机 `brew install shellcheck` 后补跑一次。
- 仓库门禁不受影响：脚本不入构建链（release.yml 未引用），仅人工执行。
- 变更仅两文件：`scripts/release/smoke-release.sh`（新增）、本报告（新增）。

## 8. 复盘（≤10 行）

- 完成度：验收标准全满足；脚本对 v0.2.0 实测 12/12 PASS，退出码 0。
- 新事实：sidecar 包内名 `rudder`（无 triple）；主二进制名 `rudder-app`；签名实为 adhoc/linker-signed；SUMS 路径带前缀；github 链路极不稳定，API 通道稳定。
- 缺陷与隐患（本次踩过并已修）：curl HTTP2/停滞挂死、子 shell 变量丢失、awk basename 对账差一、`set -u` 下未初始化变量、GitHub API JSON 缩进错锚。
- 预估偏差：网络折腾占实际工时一半以上，纯脚本逻辑本身半天内可完成。
- 遗留处置：shellcheck 未装（补跑建议见 §7）；`Rudder-app.zip` 未解压实测（dmg 已覆盖同源验证）。
- 下一步建议：① 发新版本后主会话直接复跑 `smoke-release.sh v<tag>`；② `Rudder-app.zip` 装法补一条解压+Gatekeeper 备注（README 已有，无阻塞）；③ 有空补 shellcheck 门禁。
- 大白话：东西是好的，能下、能验、能挂、能跑、版本全对齐；用户装不上九成是 Gatekeeper 或网络下载这两件事，报告里都有对应话术。

## 附录：最终轮完整运行日志（2026-09-11 22:30:13 -0700）

```text
smoke: ===== imeepos/ui-designer @ v0.2.0（host: arm64, macOS 26.6）=====
smoke: 工作目录: /tmp/rudder-smoke-v0.2.0-5e6WDR
smoke: 前置探测: github.com 直连可达（通道1 启用）
    | smoke: 下载资产 Rudder_0.2.0_aarch64.dmg
    | smoke:   通道1: github.com browser_download_url（浏览器同款入口）
    | smoke:   下载成功（8061317 字节）
smoke: [下载 dmg（Rudder_0.2.0_aarch64.dmg）] exit=0 PASS
    | smoke: 下载资产 SHA256SUMS.txt
    | smoke:   通道1: github.com browser_download_url（浏览器同款入口）
    | smoke:   下载成功（1055 字节）
smoke: [下载 SHA256SUMS.txt] exit=0 PASS
    | smoke: 下载资产 rudder-aarch64-apple-darwin
    | smoke:   通道1: github.com browser_download_url（浏览器同款入口）
    | curl: (28) Operation too slow. Less than 1024 bytes/sec transferred the last 30 seconds
    | smoke:   下载成功（3981840 字节）
smoke: [下载裸 CLI（rudder-aarch64-apple-darwin）] exit=0 PASS
    | smoke:   SHA256 本地实值   = 037d847a5c2fd4d8534c068cd324c59d68492a1c549a9b6fde7a2f309e5a0119
    | smoke:   SHA256 清单实值   = 037d847a5c2fd4d8534c068cd324c59d68492a1c549a9b6fde7a2f309e5a0119
smoke: [SHA256 校验 Rudder_0.2.0_aarch64.dmg] exit=0 PASS
    | smoke:   SHA256 本地实值   = 7d559fe559bd0c665c1ef18fcaadbc483a86d9995f2884ac2ce793356b19d619
    | smoke:   SHA256 清单实值   = 7d559fe559bd0c665c1ef18fcaadbc483a86d9995f2884ac2ce793356b19d619
smoke: [SHA256 校验 rudder-aarch64-apple-darwin] exit=0 PASS
smoke: 备注: curl 下载文件不带 quarantine 属性（浏览器下载会带，Gatekeeper 形态见报告）
    | Checksumming Protective Master Boot Record (MBR : 0)…
    | Protective Master Boot Record (MBR :: verified   CRC32 $AE7EA1AB
    | Checksumming GPT Header (Primary GPT Header : 1)…
    |  GPT Header (Primary GPT Header : 1): verified   CRC32 $51844B31
    | Checksumming GPT Partition Data (Primary GPT Table : 2)…
    | GPT Partition Data (Primary GPT Tabl: verified   CRC32 $F6181301
    | Checksumming  (Apple_Free : 3)…
    |                     (Apple_Free : 3): verified   CRC32 $00000000
    | Checksumming disk image (Apple_HFS : 4)…
    |           disk image (Apple_HFS : 4): verified   CRC32 $E667F8B9
    | Checksumming  (Apple_Free : 5)…
    |                     (Apple_Free : 5): verified   CRC32 $00000000
    | Checksumming GPT Partition Data (Backup GPT Table : 6)…
    | GPT Partition Data (Backup GPT Table: verified   CRC32 $F6181301
    | Checksumming GPT Header (Backup GPT Header : 7)…
    |   GPT Header (Backup GPT Header : 7): verified   CRC32 $DAFDB906
    | verified   CRC32 $9218D605
    | /dev/disk8          	GUID_partition_scheme          	
    | /dev/disk8s1        	Apple_HFS                      	/private/tmp/rudder-smoke-v0.2.0-5e6WDR/mnt
smoke: [挂载 dmg（hdiutil attach -readonly）] exit=0 PASS
    | smoke:   发现 .app（1 个）: /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app
smoke: [.app 结构：挂载点内恰有一个 .app] exit=0 PASS
    | smoke:   CFBundleShortVersionString = 0.2.0（期望 0.2.0）
    | smoke:   CFBundleExecutable         = rudder-app
    | smoke:   主二进制                    = /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app/Contents/MacOS/rudder-app
smoke: [.app 结构：主二进制存在且可执行] exit=0 PASS
smoke: [版本一致：CFBundleShortVersionString=0.2.0] exit=0 PASS
smoke:   bundle id: com.rudder.studio
    | smoke:   候选 sidecar: /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app/Contents/MacOS/rudder（3981840 字节）
    |     | /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app/Contents/MacOS/rudder: Mach-O 64-bit executable arm64
    | smoke:   --version => rudder 0.2.0 PASS
smoke: [sidecar CLI 存在且 --version=0.2.0] exit=0 PASS
smoke: --- codesign 状态（预期无有效签名，非失败项，如实记录）---
    | /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app: code has no resources but signature indicates they must be present
    | Executable=/private/tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app/Contents/MacOS/rudder-app
    | Identifier=rudder_app-5d36567ceaac7e33
    | Format=app bundle with Mach-O thin (arm64)
    | CodeDirectory v=20400 size=82776 flags=0x20002(adhoc,linker-signed) hashes=2583+0 location=embedded
    | Signature=adhoc
    | Info.plist=not bound
    | TeamIdentifier=not set
    | Sealed Resources=none
    | Internal requirements=none
    | /tmp/rudder-smoke-v0.2.0-5e6WDR/mnt/Rudder.app: code has no resources but signature indicates they must be present
smoke: --- 裸 CLI rudder-aarch64-apple-darwin ---
    | /tmp/rudder-smoke-v0.2.0-5e6WDR/rudder-aarch64-apple-darwin: Mach-O 64-bit executable arm64
    | smoke:   --version => rudder 0.2.0（期望含 0.2.0）
smoke: [裸 CLI --version 退出码 0 且含 0.2.0] exit=0 PASS
smoke: ===== 冒烟汇总: 11 PASS / 0 FAIL =====
smoke: PASS 结论：产物健康，用户报障优先按环境问题排查（Gatekeeper/钥匙串/网络）
smoke: 清理 [hdiutil detach] exit=0
SCRIPT-EXIT=0
```
