# 舵 Rudder · 自举计划（Self-Bootstrap）v1

> Phase 4 核心任务：用「舵」自己的 CLI+Skill 为「舵」自己生成成套设计资产，再用产出物审视并打磨工具自身。这是 Dogfooding：任何卡壳都是产品缺陷。

## 1. 设计简报（喂给自己 CLI 的"客户需求"）
- **项目名**：舵 Rudder · Studio
- **画布**：web 1536x1024
- **品牌简报**：远洋航海工作室气质，海军蓝主色 + 黄铜点缀，克制专业的工具感；大量中性色承载内容；无渐变滥用（对齐 docs/THEME.md 的产品性格）
- **总板要求**：含 hex 标签色板、Inter/PingFang 字阶、按钮/卡片/输入框样本、线性图标风格、8px 间距规则

## 2. 页面清单（等价于产品自己的三大界面）
| slug | 简报要点 |
|---|---|
| `project-library` | 左栏项目列表+新建按钮；中间空态含罗盘线稿插画；顶部四步步条 |
| `gallery` | 中间画廊网格：总板候选/页面/组件三区，卡片 hover 浮现操作；右栏详情表单 |
| `detail` | 右栏操作面板：简报表单+生成按钮+进度骨架屏+历史版本缩略图条 |

## 3. 组件清单
| name | type | 说明 |
|---|---|---|
| `button-set` | buttons | primary/secondary/ghost + hover/disabled |
| `stepper` | navigation | 四步步条：完成/当前(黄铜)/未来三态 |

## 4. 执行协议
1. 全程 `quality low` 探索 → 选定后总板+gallery 两张用 `high` 精出；预算上限 $1.5
2. 每一步若 CLI 报错/文案含糊/输出与简报偏差 → 记录到 `docs/UI-REVIEW.md` 的「工具缺陷」节（这就是打磨清单）
3. export 后按 skill 的 design-md.md 流程写 `design-assets/DESIGN.md`
4. 对照 DESIGN.md 与 docs/THEME.md 逐项比对 → 「界面改进项」节（≥5 条，标优先级）
5. Phase 5 按 UI-REVIEW 逐条整改工具自身界面

## 5. 判定
- 自举全程仅凭 SKILL.md 完成、无外部求助 = Skill 达标
- design-assets/ 下 1 总板 + 3 页 + 2 组件，目测同风格 = 锚点机制达标
- UI-REVIEW 两节齐备且可执行 = 自举达标
