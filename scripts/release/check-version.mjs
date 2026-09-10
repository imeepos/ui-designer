#!/usr/bin/env node
// 版本一致性门禁：发布流水线的第一道闸。
//
// 「舵」的版本号散落在多处清单里，任何一个漏改都会产出「GUI 与 CLI 版本错位」
// 的安装包（docs/CLI-BORROW-PLAN.md P2：三工件版本严格一致是统一下发的前提）：
//   - package.json                （前端 / tauri cli 入口）
//   - Cargo.toml                  （[workspace.package] version，core/cli/src-tauri 均继承）
//   - src-tauri/tauri.conf.json   （安装包与 updater 元数据版本）
// CLI 的 `--version` 来自 env!("CARGO_PKG_VERSION")，继承 workspace 版本，
// 因此校验以上三处即可覆盖 GUI / CLI / 包元数据。
//
// 用法：
//   node scripts/release/check-version.mjs                    # 仅校验三处一致
//   node scripts/release/check-version.mjs --expect v0.1.0    # 额外校验 tag 匹配
import { readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dirname, "..", "..");

function fail(message) {
  console.error(`version-gate: FAIL ${message}`);
  process.exit(1);
}

const pkg = JSON.parse(readFileSync(join(ROOT, "package.json"), "utf8"));
const cargoToml = readFileSync(join(ROOT, "Cargo.toml"), "utf8");
const tauriConf = JSON.parse(
  readFileSync(join(ROOT, "src-tauri", "tauri.conf.json"), "utf8"),
);

const m = cargoToml.match(/^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
if (!m) fail("Cargo.toml 未找到 [workspace.package] version");
const cargoVersion = m[1];

const versions = {
  "package.json": pkg.version,
  "Cargo.toml [workspace.package]": cargoVersion,
  "src-tauri/tauri.conf.json": tauriConf.version,
};

const distinct = [...new Set(Object.values(versions))];
if (distinct.length !== 1) {
  fail(
    `清单版本不一致:\n` +
      Object.entries(versions)
        .map(([k, v]) => `  ${k} = ${v}`)
        .join("\n"),
  );
}

const version = distinct[0];
console.log(`version-gate: PASS 三处清单一致 = ${version}`);

// --expect：tag 触发时校验 tag 名（refs/tags/v0.1.0 → GITHUB_REF_NAME = v0.1.0）
const expectIdx = process.argv.indexOf("--expect");
if (expectIdx !== -1) {
  const expected = process.argv[expectIdx + 1];
  if (!expected || !/^v\d+\.\d+\.\d+/.test(expected)) {
    fail(`--expect 需要 v<semver> 形式的 tag 名，收到：${expected ?? "(空)"}`);
  }
  if (expected !== `v${version}`) {
    fail(`tag ${expected} 与清单版本 ${version} 不匹配（应为 v${version}）`);
  }
  console.log(`version-gate: PASS tag ${expected} 与清单版本匹配`);
}
