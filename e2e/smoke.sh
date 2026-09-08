#!/usr/bin/env bash
# 舵 Rudder · e2e 冒烟：CLI 全链路验证
# 默认走真实 API（会产生少量生图费用）；DRYRUN=1 时全链路只出请求计划，零花费：
#   e2e/smoke.sh [quality]          # quality 默认 low；仅允许 low|high
#   DRYRUN=1 e2e/smoke.sh           # 计划模式：不需要 OPENAI_API_KEY
set -euo pipefail

QUALITY="${1:-low}"
case "$QUALITY" in low|high) ;; *) echo "quality 仅允许 low|high" >&2; exit 1;; esac

DRYRUN="${DRYRUN:-0}"
case "$DRYRUN" in 0|1) ;; *) echo "DRYRUN 仅允许 0|1" >&2; exit 1;; esac

if [ "$DRYRUN" = "0" ]; then
  : "${OPENAI_API_KEY:?请先 export OPENAI_API_KEY}"
fi
: "${OPENAI_BASE_URL:=https://api.openai.com}"
export OPENAI_API_KEY OPENAI_BASE_URL

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/rudder"
WORK="$(mktemp -d /tmp/rudder-e2e.XXXXXX)"
BUDGET_CNY="${BUDGET_CNY:-15}"

# 计划模式旗标：--dry-run 与 --yes 同时存在时 CLI 以 --dry-run 优先（免费）
PLAN_FLAG=()
if [ "$DRYRUN" = "1" ]; then
  PLAN_FLAG=(--dry-run)
fi

echo "== e2e 工作目录: $WORK (quality=$QUALITY, dryrun=$DRYRUN) =="

run() { echo "\$ $*"; "$@"; }

run "$BIN" init "e2e样例" --size web --dir "$WORK/proj" --brief "远洋航运 SaaS，海军蓝+黄铜点缀，克制专业"

# 总板：low 探索价，n=2 控制成本
run "$BIN" board generate --n 2 --quality "$QUALITY" --yes ${PLAN_FLAG[@]+"${PLAN_FLAG[@]}"} --project "$WORK/proj"
if [ "$DRYRUN" = "0" ]; then
  ANCHOR="$(ls "$WORK/proj/board/candidates" | head -1)"
else
  # 计划模式没有真实产物：造一张占位图让 pick 的文件操作链路可继续
  mkdir -p "$WORK/proj/board/candidates"
  head -c 20480 /dev/zero > "$WORK/proj/board/candidates/0001.png"
  ANCHOR="0001.png"
fi
run "$BIN" board pick "$ANCHOR" --project "$WORK/proj"

run "$BIN" page add dashboard --brief "运营仪表盘：顶部指标卡x4，中部折线图区，右侧任务列表" --project "$WORK/proj"
run "$BIN" page generate dashboard --quality "$QUALITY" --yes ${PLAN_FLAG[@]+"${PLAN_FLAG[@]}"} --project "$WORK/proj"
if [ "$DRYRUN" = "0" ]; then
  PAGE_CAND="$(ls "$WORK/proj/pages/dashboard/candidates" | head -1)"
else
  mkdir -p "$WORK/proj/pages/dashboard/candidates"
  head -c 20480 /dev/zero > "$WORK/proj/pages/dashboard/candidates/0001.png"
  PAGE_CAND="0001.png"
fi
run "$BIN" page pick dashboard "$PAGE_CAND" --project "$WORK/proj"

run "$BIN" component add button-set --type buttons --brief "主/次/幽灵按钮三态" --project "$WORK/proj"
run "$BIN" component generate button-set --quality "$QUALITY" --yes ${PLAN_FLAG[@]+"${PLAN_FLAG[@]}"} --project "$WORK/proj"
if [ "$DRYRUN" = "0" ]; then
  COMP_CAND="$(ls "$WORK/proj/components/button-set/candidates" | head -1)"
else
  mkdir -p "$WORK/proj/components/button-set/candidates"
  head -c 20480 /dev/zero > "$WORK/proj/components/button-set/candidates/0001.png"
  COMP_CAND="0001.png"
fi
run "$BIN" component pick button-set "$COMP_CAND" --project "$WORK/proj"

run "$BIN" export --out "$WORK/export" --project "$WORK/proj"

echo "== 产物校验 =="
test -f "$WORK/proj/board/anchor.png"
test -f "$WORK/export/manifest.json"
test -f "$WORK/export/PROMPTS.md"
test -f "$WORK/export/DESIGN.template.md"

if [ "$DRYRUN" = "0" ]; then
  test -f "$WORK/proj/pages/dashboard/current.png"
  test -f "$WORK/proj/components/button-set/current.png"
  python3 - "$WORK/export/manifest.json" <<'PY'
import json,sys,os
m=json.load(open(sys.argv[1]))
assert m["project"]["name"]=="e2e样例", m.keys()
assert len(m["pages"])>=1 and len(m["components"])>=1
for rel in [m["board"]["anchor"]["file"]] + [p["file"] for p in m["pages"]]:
    assert os.path.getsize(os.path.join(os.path.dirname(sys.argv[1]), rel))>10000, rel
print("manifest OK:", m["project"]["name"], "pages:",len(m["pages"]))
PY
else
  # 计划模式：无真实图片，只校验 manifest 结构与每步计划落盘
  python3 - "$WORK/export/manifest.json" <<'PY'
import json,sys
m=json.load(open(sys.argv[1]))
assert m["project"]["name"]=="e2e样例", m.keys()
assert m["project"]["canvasSize"]["preset"]=="web"
print("manifest(dryrun) OK:", m["project"]["name"])
PY
  echo "== 计划模式提示：真实 API 冒烟请不带 DRYRUN 重新执行 =="
fi

echo "== PASS: e2e 全链路通过 (dryrun=$DRYRUN) =="
