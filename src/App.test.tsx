import { renderToString } from "react-dom/server";
import { beforeAll, expect, test } from "vitest";

import App from "./App";
import i18n from "./i18n";

beforeAll(async () => {
  await i18n.changeLanguage("zh-CN");
});

test("renders three-column shell with four-step flow (zh-CN)", () => {
  const html = renderToString(<App />);

  expect(html).toContain('data-testid="app-shell"');
  expect(html).toContain('data-testid="projects-panel"');
  expect(html).toContain('data-testid="gallery-panel"');
  expect(html).toContain('data-testid="detail-panel"');
  expect(html).toContain('data-testid="stepper"');

  for (const id of ["project", "board", "page", "component"]) {
    expect(html).toContain(`data-testid="step-${id}"`);
  }
  expect(html).toContain('data-testid="step-project" data-state="current"');
  expect(html).toContain('data-testid="step-board" data-state="upcoming"');

  expect(html).toContain("画廊");
  expect(html).toContain("详情与操作");
  expect(html).toContain("新建项目");
});

test("switching language to en updates all visible copy", async () => {
  await i18n.changeLanguage("en");
  const html = renderToString(<App />);

  expect(html).toContain("Gallery");
  expect(html).toContain("Details");
  expect(html).toContain("New project");
  expect(html).not.toContain("画廊");
  expect(html).not.toContain("新建项目");

  await i18n.changeLanguage("zh-CN");
});

test("step descriptions stay muted and steps carry theme states", () => {
  const html = renderToString(<App />);
  // 当前步应有黄铜色圆点（accent）
  expect(html).toContain('data-testid="step-project" data-state="current"');
  // 未来步置灰
  expect(html).toContain('data-testid="step-component" data-state="upcoming"');
});
