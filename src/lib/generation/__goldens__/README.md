# prompt golden 快照（Rust 引擎生成，TS 引擎对拍）

本目录的 10 个 `.txt` 是 **Rust 引擎**（`crates/rudder-core/src/prompt.rs`
`mod goldens`）对固定 fixture 集合（每个内置模板 × 2 组代表性变量组合）
合成的 prompt 原文，是 CLI/桌面双引擎对拍的**唯一真源**：

- **生成（仅在有意的引擎/模板改动后）**：
  `RUDDER_UPDATE_GOLDENS=1 cargo test -p rudder-core --lib goldens`
- **消费**：`src/lib/generation/prompt.golden.test.ts`（vitest）断言 TS 引擎
  输出与这些文件**逐字节一致**；任何一边漂移都会挂测试。
- **不要手编**这些文件；它们只由上述命令生成。
- 改 fixture（`fixture_a`/`fixture_b`）或模板时，必须在**同一提交**里同步
  Rust 侧（`prompt.rs mod goldens`）与 TS 侧（`prompt.golden.test.ts`）的
  字面量，再重新生成本目录。

命名：`<template-id>--<a|b>.txt`（a=完整规格组，b=边界组：trim、空 style、
negativeHints、自定义竖版画布、逐字 Labels）。
