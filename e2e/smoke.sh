#!/usr/bin/env bash
# 舵 Rudder · e2e 冒烟：CLI 全链路真实 API 验证（会产生少量生图费用）
# 用法: e2e/smoke.sh [quality]   # quality 默认 low；仅允许 low|high
set -euo pipefail

QUALITY="${1:-low}"
case "$QUALITY" in low|high) ;; *) echo "quality 仅允许 low|high" >&2; exit 1;; esac

: "${OPENAI_API_KEY:?请先 export OPENAI_API_KEY}"
: "${OPENAI_BASE_URL:=https://api.openai.com}"
export OPENAI_API_KEY OPENAI_BASE_URL

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/debug/rudder"
WORK="$(mktemp -d /tmp/rudder-e2e.XXXXXX)"
BUDGET_CNY="${BUDGET_CNY:-15}"

echo "== e2e 工作目录: $WORK (quality=$QUALITY) =="

run() { echo "\$ $*"; "$@"; }

run "$BIN" init "e2e样例" --size web --dir "$WORK/proj" --brief "远洋航运 SaaS，海军蓝+黄铜点缀，克制专业"

# 总板：low 探索价，n=2 控制成本
run "$BIN" board generate --n 2 --quality "$QUALITY" --yes --project "$WORK/proj"
ANCHOR="$(ls "$WORK/proj/board/candidates" | head -1)"
run "$BIN" board pick "$ANCHOR" --project "$WORK/proj"

run "$BIN" page add dashboard --brief "运营仪表盘：顶部指标卡x4，中部折线图区，右侧任务列表" --project "$WORK/proj"
run "$BIN" page generate dashboard --quality "$QUALITY" --yes --project "$WORK/proj"

run "$BIN" component add button-set --type buttons --brief "主/次/幽灵按钮三态" --project "$WORK/proj"
run "$BIN" component generate button-set --quality "$QUALITY" --yes --project "$WORK/proj"

run "$BIN" export --out "$WORK/export" --project "$WORK/proj"

echo "== 产物校验 =="
test -f "$WORK/proj/board/anchor.png"
test -f "$WORK/proj/pages/dashboard/current.png"
test -f "$WORK/proj/components/button-set/current.png"
test -f "$WORK/export/manifest.json"
test -f "$WORK/export/PROMPTS.md"
python3 - "$WORK/export/manifest.json" <<'PY'
import json,sys,os
m=json.load(open(sys.argv[1]))
assert m["project"]["name"]=="e2e样例", m.keys()
assert len(m["pages"])>=1 and len(m["components"])>=1
for rel in [m["board"]["anchor"]["file"]] + [p["file"] for p in m["pages"]]:
    assert os.path.getsize(os.path.join(os.path.dirname(sys.argv[1]), rel))>10000, rel
print("manifest OK:", m["project"]["name"], "pages:",len(m["pages"]))
PY

echo "== PASS: e2e 全链路通过 =="
