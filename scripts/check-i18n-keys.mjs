#!/usr/bin/env node
// Static check: every t("...") key used in src must exist in BOTH i18n
// resources, and zh-CN/en must stay in parity. Dynamic template keys are
// validated by subtree prefix (e.g. `component.type.${type}`).
import { readdirSync, readFileSync } from "node:fs";
import { extname, join, relative } from "node:path";

const ROOT = join(import.meta.dirname, "..");
const SRC = join(ROOT, "src");
const SKIP_DIRS = new Set(["i18n"]);
const SKIP_FILE = (name) =>
  name.endsWith(".test.ts") || name.endsWith(".test.tsx") || name.endsWith(".d.ts");
const EXTENSIONS = new Set([".ts", ".tsx"]);

const zh = JSON.parse(readFileSync(join(SRC, "i18n", "zh-CN.json"), "utf8"));
const en = JSON.parse(readFileSync(join(SRC, "i18n", "en.json"), "utf8"));

function collect(dir, out = []) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) collect(join(dir, entry.name), out);
    } else if (!SKIP_FILE(entry.name) && EXTENSIONS.has(extname(entry.name))) {
      out.push(join(dir, entry.name));
    }
  }
  return out;
}

function lookup(resource, key) {
  let node = resource;
  for (const part of key.split(".")) {
    if (node === null || typeof node !== "object" || !(part in node)) return undefined;
    node = node[part];
  }
  return node;
}

function hasLeafUnder(resource, prefix) {
  const node = lookup(resource, prefix);
  if (node === undefined) return false;
  if (typeof node === "string") return true;
  return Object.values(node).some(
    (value) => typeof value === "string" || (value !== null && typeof value === "object"),
  );
}

function leaves(resource, prefix = "") {
  const out = [];
  for (const [key, value] of Object.entries(resource)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === "object") out.push(...leaves(value, path));
    else out.push(path);
  }
  return out;
}

const files = collect(SRC);
const missing = [];
const dynamicPrefixes = [];

const PLAIN = /(?:^|[^\w$.])t\(\s*(["'])((?:\\.|(?!\1).)+?)\1/gs;
const TEMPLATE = /\bt\(\s*`([^`]+)`/g;

for (const file of files) {
  const source = readFileSync(file, "utf8");
  const rel = relative(ROOT, file);

  for (const match of source.matchAll(PLAIN)) {
    const key = match[2].replace(/\\'/g, "'");
    if (key.includes("${")) continue;
    if (lookup(zh, key) === undefined) missing.push(`zh-CN missing: ${key} (${rel})`);
    if (lookup(en, key) === undefined) missing.push(`en missing: ${key} (${rel})`);
  }

  for (const match of source.matchAll(TEMPLATE)) {
    const raw = match[1];
    const prefix = raw.split("${")[0].replace(/\.$/, "");
    if (!prefix) continue;
    dynamicPrefixes.push(prefix);
    if (!hasLeafUnder(zh, prefix)) missing.push(`zh-CN missing subtree: ${prefix} (${rel})`);
    if (!hasLeafUnder(en, prefix)) missing.push(`en missing subtree: ${prefix} (${rel})`);
  }
}

// zh/en parity: same leaf key sets.
const zhLeaves = new Set(leaves(zh));
const enLeaves = new Set(leaves(en));
for (const key of zhLeaves) if (!enLeaves.has(key)) missing.push(`parity: en lacks ${key}`);
for (const key of enLeaves) if (!zhLeaves.has(key)) missing.push(`parity: zh-CN lacks ${key}`);

if (missing.length > 0) {
  console.error(`[lint:i18n] found ${missing.length} key problem(s):`);
  for (const item of missing) console.error(`  ${item}`);
  process.exit(1);
}
console.log(
  `[lint:i18n] OK - ${zhLeaves.size} keys in parity across zh-CN/en; ` +
    `${dynamicPrefixes.length} dynamic prefix(es) verified in ${files.length} file(s)`,
);
