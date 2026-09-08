#!/usr/bin/env node
// Scan frontend sources for hardcoded Chinese UI copy.
// Discipline (AGENTS.md): UI copy lives ONLY in src/i18n/*.json.
import { readdirSync, readFileSync } from "node:fs";
import { extname, join, relative } from "node:path";

const SRC_ROOT = join(import.meta.dirname, "..", "src");
const CJK = /[\u3400-\u9fff\uf900-\ufaff]/;
const SKIP_DIRS = new Set(["i18n"]);
const SKIP_FILE = (name) =>
  name.endsWith(".test.ts") || name.endsWith(".test.tsx") || name.endsWith(".d.ts");
const EXTENSIONS = new Set([".ts", ".tsx"]);

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

const violations = [];
for (const file of collect(SRC_ROOT)) {
  const lines = readFileSync(file, "utf8").split("\n");
  lines.forEach((line, i) => {
    if (CJK.test(line)) {
      violations.push(`${relative(SRC_ROOT, file)}:${i + 1}: ${line.trim()}`);
    }
  });
}

if (violations.length > 0) {
  console.error(`[lint:text] found ${violations.length} hardcoded copy line(s) outside src/i18n:`);
  for (const v of violations) console.error(`  ${v}`);
  process.exit(1);
}
console.log("[lint:text] OK - no hardcoded CJK copy in src (i18n resources excluded)");
