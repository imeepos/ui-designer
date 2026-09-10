#!/usr/bin/env node
// sidecar 准备脚本：把 rudder-cli 的 release 二进制按 Tauri sidecar 命名约定落位。
//
// Tauri `bundle.externalBin: ["binaries/rudder"]` 要求构建期在 src-tauri/binaries/
// 下存在带 host target triple 后缀的二进制（Windows 额外 .exe）：
//   macOS   → binaries/rudder-aarch64-apple-darwin
//   Linux   → binaries/rudder-x86_64-unknown-linux-gnu
//   Windows → binaries/rudder-x86_64-pc-windows-msvc.exe
// 本脚本对 host triple 做运行时探测（rustc -vV），不硬编码，矩阵换 runner 不用改脚本。
//
// 落位后立即执行 `--version` 冒烟，把「能跑 + 版本与清单一致」提前到打包前暴露
// （docs/CLI-BORROW-PLAN.md P2 验收：sidecar 二进制版本三工件一致）。
//
// 用法：
//   node scripts/release/prepare-sidecar.mjs [--skip-build]
//     --skip-build  跳过 cargo build，仅复用 target/release 已有二进制（调试用）
import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

const ROOT = join(import.meta.dirname, "..", "..");
const BINARIES_DIR = join(ROOT, "src-tauri", "binaries");

function fail(message) {
  console.error(`sidecar: FAIL ${message}`);
  process.exit(1);
}

function run(cmd, args, opts = {}) {
  const res = spawnSync(cmd, args, { encoding: "utf8", ...opts });
  if (res.error || res.status !== 0) {
    fail(
      `\`${cmd} ${args.join(" ")}\` ` +
        (res.error
          ? `spawn 失败：${res.error.message}（检查 ${cmd} 是否在 PATH）`
          : `退出码 ${res.status}`) +
        `\n${res.stdout ?? ""}${res.stderr ?? ""}`,
    );
  }
  return res;
}

function hostTriple() {
  const res = run("rustc", ["-vV"]);
  const m = res.stdout.match(/host:\s*(\S+)/);
  if (!m) fail(`rustc -vV 输出未找到 host triple：\n${res.stdout}`);
  return m[1];
}

// ---------- 1. 构建 CLI ----------
const skipBuild = process.argv.includes("--skip-build");
if (!skipBuild) {
  console.log("sidecar: cargo build --release -p rudder-cli ...");
  run("cargo", ["build", "--release", "-p", "rudder-cli"], { cwd: ROOT });
}

const exe = process.platform === "win32" ? ".exe" : "";
const built = join(ROOT, "target", "release", `rudder${exe}`);
if (!existsSync(built)) {
  fail(`构建产物不存在：${built}（--skip-build 时需先自行构建）`);
}

// ---------- 2. triple 命名落位 ----------
const triple = hostTriple();
const sidecarName = `rudder-${triple}${exe}`;
mkdirSync(BINARIES_DIR, { recursive: true });
copyFileSync(built, join(BINARIES_DIR, sidecarName));
console.log(`sidecar: ${built} -> src-tauri/binaries/${sidecarName}`);

// ---------- 3. --version 冒烟 ----------
const smoke = spawnSync(join(BINARIES_DIR, sidecarName), ["--version"], {
  encoding: "utf8",
});
if (smoke.status !== 0) {
  fail(`sidecar 二进制 --version 退出码 ${smoke.status}\n${smoke.stderr ?? ""}`);
}
const printed = (smoke.stdout ?? "").trim();
const vm = printed.match(/(\d+\.\d+\.\d+)/);
if (!vm) fail(`--version 输出无法解析版本号：${printed}`);

// 与 tauri.conf.json 对账（CLI 版本继承 workspace，GUI 清单是发布面元数据）
const tauriConf = JSON.parse(
  readFileSync(join(ROOT, "src-tauri", "tauri.conf.json"), "utf8"),
);
if (vm[1] !== tauriConf.version) {
  fail(`CLI 版本 ${vm[1]} 与 tauri.conf.json 版本 ${tauriConf.version} 不一致`);
}

console.log(`sidecar: PASS ${sidecarName} --version => ${printed}（与 GUI 清单一致）`);
