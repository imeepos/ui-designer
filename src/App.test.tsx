import { renderToString } from "react-dom/server";
import { beforeAll, expect, test } from "vitest";

import App from "./App";
import i18n from "./i18n";

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

test("renders three-column shell with directory tree (zh-CN)", () => {
  const html = renderToString(<App />);

  expect(html).toContain('data-testid="app-shell"');
  expect(html).toContain('data-testid="projects-panel"');
  expect(html).toContain('data-testid="gallery-panel"');
  expect(html).toContain('data-testid="detail-panel"');

  // 横向步条已删除，取而代之的是目录树。
  expect(html).not.toContain('data-testid="stepper"');
  expect(html).toContain('data-testid="project-tree"');

  // 无项目：树只显示新建入口。
  expect(html).toContain('data-testid="tree-empty-create"');
  expect(html).not.toContain('data-testid="tree-root"');

  expect(html).toContain("画廊");
  expect(html).toContain("详情与操作");
  expect(html).toContain("新建项目");
  expect(html).toContain("目录");
});

test("switching language to en updates all visible copy", async () => {
  await i18n.changeLanguage("en");
  const html = renderToString(<App />);

  expect(html).toContain("Gallery");
  expect(html).toContain("Details");
  expect(html).toContain("New project");
  expect(html).toContain("Library");
  expect(html).not.toContain("画廊");
  expect(html).not.toContain("新建项目");

  await i18n.changeLanguage("zh-CN");
});

test("stepper markup is fully gone from the shell", () => {
  const html = renderToString(<App />);
  expect(html).not.toContain('data-testid="step-project"');
  expect(html).not.toContain('data-testid="step-nav-');
  expect(html).not.toContain("设计流程");
});
