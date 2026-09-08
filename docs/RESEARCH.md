# 舵 Rudder · 开源竞品调研报告（2026-09-09）

> 由调研子代理产出，项目负责人归档。结论已合入 ARCHITECTURE.md（标 🆕 的条目）。

## 一、逐产品要点

| 产品 | 定位 | 借鉴 | 不借鉴 |
|---|---|---|---|
| **Onlook** | 设计师的 Cursor，React DOM 可视化编辑（曾用 Tauri） | 分支实验、checkpoint 版本史、品牌资产+token 面板 | 深绑 Next.js、桌面跑编译器过重 |
| **screenshot-to-code** | 截图转代码 | 输入输出并排+预览验证；模型/框架可切换管道 | 单向转换，无项目沉淀 |
| **OpenUI (W&B)** | 聊天式文字→UI | 对话迭代所见即所得；BYOK/本地模型 | 无画布、无成套 |
| **tldraw make-real** | 手绘→GPT-4V→网页 | 画布即对话空间：生成物+批注共存，迭代=在旧结果上标注；结果回画布 | 每轮全量重生成、无设计系统 |
| **Penpot** | 开源 Figma 替代 | 开放 token/组件数据格式；插件 API；项目→文件→画板层级 | 全功能设计器体量 |
| **Quant-UX** | 原型+可用性测试 | 页面树/流程图组织 | 测试业务线 |
| **Akira UX** | Vala+GTK 设计工具（⚠️非 Rust；Rust 同类=Graphite） | 原生桌面路线印证 Tauri 可行 | 功能停滞=纯画布无 AI 的反面教材 |
| **Excalidraw** | 手绘白板 | 场景文件 JSON 可 diff；结构化文本→图元 | 手绘美学定位 |
| **Plasmic** | 可视化→生产 React | 产物即可信代码 | 面向建站非设计探索 |
| **bolt.diy / openv0** | AI 全栈生成 / v0 复刻 | artifact 流式工作台；生成沉淀为组件库 | 运行时复杂度、完成度低 |
| **superdesign** ⭐ | IDE 内设计 agent，一次多稿 mockup，Fork 迭代，落盘 `.superdesign/`；有 skill 形态 | **多稿探索哲学**；产物目录化；skill 供编码代理调用 | 依赖 IDE 宿主 |
| **OpenDesign** ⭐ | 本地优先桌面 AI 设计，Home→Plugins→Design System(→DESIGN.md 品牌契约)→Studio；skill+CLI+MCP 三通道 | DESIGN.md 契约先行；agent 多通道接入 | 导出 HTML/PPTX 等过宽范围 |

## 二、吸收结论（已合入架构）
1. **多稿探索是一等公民**：总板/页面/组件生成均支持 `--n` 多候选（同 batch 天然同风格），选中即 Fork 主稿。🆕
2. **DESIGN.md 品牌契约**：export 产出 `DESIGN.template.md`（项目元数据+图片路径+待填 token 表），由 Skill 指导 AI 代理看图填 token。🆕
3. **CLI 纪律**：stdout=数据(--json)、stderr=日志；错误结构 `{code,message,hint}`；退出码 1 参数/2 API/3 状态。🆕
4. **产物路径稳定可预测**（`board/candidates/`、`pages/<slug>/`），manifest 记录 prompt/model/seed/size 保证可复现。
5. **版本=历史快照 + Fork**（history/ 目录），生图参数随版本存档。
6. **差异化定位**：唯一「设计系统优先」的开源桌面 UI 设计图生成器——总板→页面→组件成套出图，每张图既是给人看的设计，也是给 AI 编码代理的契约输入。

## 三、v0.1 明确不跟进（记入路线图候选）
- MCP server（agent 第三通道）、无限画布、可用性测试、团队协作、HTML 导出。
