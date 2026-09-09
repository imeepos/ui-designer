# templates/ — 舵 Rudder 提示词模板资产包

代理可消费的**模板协议**：每套模板 = 参数化骨架（skeleton）+ 逐槽填槽指南（fillGuide）。
舵本身不接大模型——外部编码代理读骨架与指南后，**用自己的 LLM 填槽**，把填好的最终提示词
经 `rudder <kind> generate --prompt-file <file>` 回灌生图。

## 首批 5 套

| id | 适用产物 | 一句话 |
|---|---|---|
| `board-design-system` | board | 五分区规格板（默认锚点骨架的参数化版） |
| `page-ui-standard` | page | 标准功能页：锚点引用+布局简报+五步结构法 |
| `page-landing-sections` | page | 落地页：sections 逐段定义+导航逐项 |
| `component-sheet-grid` | component | 组件细节图：网格规格+变体×状态+小尺寸可读 |
| `brand-identity-lite` | board | 品牌身份板：关键词先行+纯白背景+3 条禁用规则 |

## 代理工作流（三步）

```bash
rudder templates list --json                 # 1. 看清单与槽位
rudder templates show page-ui-standard --json # 2. 读骨架 + fillGuide（用你的 LLM 填槽）
#    把填好的最终提示词写入 /tmp/page.prompt
rudder page generate dashboard --prompt-file /tmp/page.prompt --template page-ui-standard --dry-run
```

- `--prompt-file`：文件内容**原样**作为本次生图 prompt（引擎不改写、不注入 constraints）。
- `--template`：配合 `--prompt-file` 仅作血缘记录（记入 promptLog/manifest，保证可复现）；不带
  `--prompt-file` 时它切换引擎拼装骨架（显式 `--template` > `project.templateId` > 内置默认）。

## 槽位语法与规则

- 骨架中的 `{project.name}`、`{page.brief}` 等为槽位，词表见 `manifest.json` 的 `slotVocabulary`。
- 引擎拼装路径（默认/`--template`）下：槽位值全空的**整行自动省略**；未知槽位报错（退出码 1）。
- 页面/组件简报里可写 `Labels: a|b|c` 行声明逐字标签，引擎自动追加「逐字渲染」约束。

## 来源与许可（attribution）

骨架由舵 Rudder 原创改写；**结构模式**（分区法、计数显式、逐字文本、负面清单、纯白背景、
3 条禁用规则等）提炼自 [awesome-gpt-image-2](https://github.com/freestylefly/awesome-gpt-image-2)
（MIT 许可）。**任何画廊案例原文均未逐字内嵌**——蒸馏映射见
`docs/GPT-IMAGE-2-DESIGN-KNOWLEDGE.md`。
