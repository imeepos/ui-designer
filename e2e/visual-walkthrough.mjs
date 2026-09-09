// 视觉走查：无头浏览器驱动 mock 模式，沿左栏目录树走完整流程并逐屏截图。
// 路径：空态 → 建项目(自动选中总览) → 总板生成+设锚 → 页面组添加/生成/转正
//       → 组件组同 → 暗色/英文 → 无锚项目门控 toast（置灰引导）。
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.BASE_URL ?? "http://localhost:4173/";
const OUT = "e2e/screenshots";
mkdirSync(OUT, { recursive: true });

const shot = (page, name) => page.screenshot({ path: `${OUT}/${name}.png`, fullPage: false });

const run = async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const log = (...a) => console.log("[walk]", ...a);
  const expectTreeState = async (testId, state) => {
    const actual = await page.getByTestId(testId).getAttribute("data-state");
    if (actual !== state) throw new Error(`tree ${testId}: expect data-state=${state}, got ${actual}`);
  };

  await page.goto(BASE, { waitUntil: "networkidle" });
  await page.getByTestId("tree-empty-create").waitFor();
  await shot(page, "01-empty-zh-light");
  log("01 empty ok, api-mode:", await page.getByTestId("api-mode").textContent().catch(() => "n/a"));

  // 步骤1：新建项目 → 自动选中「总览」
  await page.getByTestId("new-project").click();
  await page.getByTestId("project-name-input").fill("远洋航运 SaaS");
  await page.getByTestId("project-brief-input").fill("深海航行工作室气质，海军蓝主色+琥珀金点缀，克制专业");
  await page.getByTestId("create-project-submit").click();
  await page.getByTestId("board-section").waitFor();
  await page.getByTestId("tree-root").waitFor();
  await expectTreeState("tree-group-overview", "current");
  await expectTreeState("tree-group-pages", "locked");
  await expectTreeState("tree-group-components", "locked");
  await shot(page, "02-overview-auto-selected");
  log("02 project created, tree auto-selects overview; pages/components locked");

  // 步骤2：总览 = 总板视图（锚点条 + 候选网格 + 右栏生成入口）
  await page.getByTestId("board-generate").click();
  await page.locator('[data-testid^="board-candidate-"]').first().waitFor();
  await shot(page, "03-board-candidates");
  await page.locator('[data-testid^="board-candidate-"] [data-testid="action-set-anchor"]').first().click();
  await page.getByTestId("anchor-badge").first().waitFor();
  await shot(page, "04-anchor-picked");
  await expectTreeState("tree-group-pages", "default");
  log("03-04 board generated + anchor picked; pages group unlocked (no forced jump)");

  // 步骤3：点「页面」组 → 组视图；行尾「+」打开添加表单
  await page.getByTestId("tree-group-open-pages").click();
  await page.getByTestId("page-section").waitFor();
  await expectTreeState("tree-group-pages", "current");
  await shot(page, "05-pages-group-empty");
  await page.getByTestId("tree-add-pages").click();
  await page.getByTestId("add-page-form").waitFor();
  await page.getByTestId("page-slug-input").fill("dashboard");
  await page.locator('[data-testid="add-page-form"] textarea').first().fill("顶部4张指标卡，中部折线图，右侧任务列表");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("tree-page-dashboard").waitFor();
  await expectTreeState("tree-page-dashboard", "current");
  await shot(page, "06-page-added-tree-selected");
  log("05-06 page group + tree-add flow; tree auto-selects page:dashboard");

  // 页面生成 + 转正
  await page.getByTestId("page-generate").first().click();
  await page.locator('[data-testid^="page-candidate-"]').first().waitFor();
  await shot(page, "07-page-candidates");
  await page.locator('[data-testid^="page-candidate-"] [data-testid="action-pick"]').first().click();
  await page.locator('[data-testid$="picked-badge"]').first().waitFor();
  await page.waitForTimeout(300);
  await shot(page, "08-page-picked");
  await expectTreeState("tree-group-components", "default");
  log("07-08 page generated + picked; components group unlocked");

  // 步骤4：组件组同路径
  await page.getByTestId("tree-group-open-components").click();
  await page.getByTestId("component-section").waitFor();
  await page.getByTestId("tree-add-components").click();
  await page.getByTestId("add-component-form").waitFor();
  await page.getByTestId("component-name-input").fill("button-set");
  await page.getByTestId("component-brief-input").fill("主/次/幽灵按钮，含悬停与禁用态");
  await page.getByTestId("add-component-submit").click();
  await page.getByTestId("tree-component-button-set").waitFor();
  await expectTreeState("tree-component-button-set", "current");
  await page.getByTestId("component-generate").first().click();
  await page.locator('[data-testid^="component-candidate-"]').first().waitFor();
  await page.locator('[data-testid^="component-candidate-"] [data-testid="action-pick"]').first().click();
  await page.locator('[data-testid$="picked-badge"]').first().waitFor();
  await page.waitForTimeout(300);
  await shot(page, "09-component-picked");
  log("09 component generated + picked via tree entry");

  // 暗色 + 英文
  await page.getByTestId("theme-toggle").click();
  await page.waitForTimeout(200);
  await shot(page, "10-dark");
  await page.getByTestId("language-switcher").getByRole("button", { name: "EN" }).click();
  await page.waitForTimeout(200);
  await shot(page, "11-dark-en");
  log("10-11 dark + en done");

  // 门控负例：新建一个未设锚项目，点置灰「页面」组 → toast 引导（不跳转）
  await page.getByTestId("new-project").click();
  await page.getByTestId("project-name-input").fill("尚未设锚的试验项目");
  await page.getByTestId("create-project-submit").click();
  await page.getByTestId("board-section").waitFor();
  await expectTreeState("tree-group-pages", "locked");
  // aria-disabled 节点仍可接收点击（用于弹出引导 toast），Playwright 需 force。
  await page.getByTestId("tree-group-open-pages").click({ force: true });
  await page.getByTestId("toast-error").waitFor();
  await shot(page, "12-tree-locked-toast");
  const toastText = await page.getByTestId("toast-error").textContent();
  if (!toastText.includes("ANCHOR_REQUIRED")) throw new Error(`locked toast missing code: ${toastText}`);
  log("12 locked pages group guides with ANCHOR_REQUIRED toast");

  await browser.close();
  console.log("[walk] PASS: 12 screenshots in", OUT);
};

run().catch((e) => { console.error("[walk] FAIL:", e.message); process.exit(1); });
