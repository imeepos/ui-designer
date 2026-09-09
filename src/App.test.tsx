import { renderToString } from "react-dom/server";
import { beforeAll, expect, test } from "vitest";

import App from "./App";
import i18n from "./i18n";

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

test("renders the home project list shell (zh-CN)", () => {
  const html = renderToString(<App />);

  expect(html).toContain('data-testid="app-shell"');
  expect(html).toContain('data-testid="home-view"');
  expect(html).toContain('data-testid="project-search"');
  expect(html).toContain('data-testid="new-project"');

  // 无项目不进入工作区；旧三栏/步条/树全部退役。
  expect(html).not.toContain('data-testid="workspace-view"');
  expect(html).not.toContain('data-testid="stepper"');
  expect(html).not.toContain('data-testid="left-menu"');
  expect(html).not.toContain('data-testid="project-tree"');

  expect(html).toContain("新建项目");
  expect(html).toContain("还没有项目");
});

test("switching language to en updates all visible copy", async () => {
  await i18n.changeLanguage("en");
  const html = renderToString(<App />);

  expect(html).toContain("New project");
  expect(html).toContain("No projects yet");
  expect(html).not.toContain("新建项目");
  expect(html).not.toContain("还没有项目");

  await i18n.changeLanguage("zh-CN");
});

test("legacy stepper and tree markup are fully gone", () => {
  const html = renderToString(<App />);
  expect(html).not.toContain('data-testid="step-project"');
  expect(html).not.toContain('data-testid="step-nav-');
  expect(html).not.toContain('data-testid="tree-root"');
});
