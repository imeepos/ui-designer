// 视觉走查：无头浏览器驱动 mock 模式四步流程，逐屏截图供负责人审美终审
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

  await page.goto(BASE, { waitUntil: "networkidle" });
  await shot(page, "01-empty-zh-light");
  log("01 empty state ok, api-mode:", await page.getByTestId("api-mode").textContent().catch(() => "n/a"));

  // 步骤1：新建项目（应用创建后自动进入总板步）
  await page.getByTestId("new-project").click();
  await page.getByTestId("project-name-input").fill("远洋航运 SaaS");
  await page.getByTestId("project-brief-input").fill("深海航行工作室气质，海军蓝主色+琥珀金点缀，克制专业");
  await page.getByTestId("create-project-submit").click();
  await page.getByTestId("board-section").waitFor();
  await shot(page, "02-project-created");
  log("02 project created, auto-advanced to board step");

  // 步骤2：总板（已在总板步；重新生成一组候选）
  await page.getByTestId("board-generate").click();
  await page.getByTestId("board-candidates").waitFor();
  await shot(page, "03-board-candidates");
  await page.locator('[data-testid^="board-candidate-"]').first().waitFor();
  await shot(page, "03-board-candidates");
  await page.locator('[data-testid^="board-candidate-"] [data-testid="action-set-anchor"]').first().click();
  await page.getByTestId("anchor-badge").waitFor();
  await shot(page, "04-anchor-picked");
  log("03-04 board generated + anchor picked");

  // 步骤3：页面
  await page.getByTestId("step-nav-page").click();
  await page.getByTestId("add-page-toggle").click();
  await page.getByTestId("page-slug-input").fill("dashboard");
  await page.locator('[data-testid="add-page-form"] textarea, [data-testid="add-page-form"] input[type="text"]').last().fill("顶部4张指标卡，中部折线图，右侧任务列表");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("page-generate").first().click();
  await page.locator('[data-testid^="page-candidate-"]').first().waitFor();
  await shot(page, "05-page-candidates");
  await page.locator('[data-testid^="page-candidate-"] [data-testid="action-pick"]').first().click();
  await page.locator('[data-testid$="picked-badge"]').first().waitFor();
  await page.waitForTimeout(300);
  await shot(page, "06-page-picked");
  log("05-06 page generated + picked");

  // 步骤4：组件
  await page.getByTestId("step-nav-component").click();
  await page.getByTestId("add-component-toggle").click();
  await page.getByTestId("component-name-input").fill("button-set");
  await page.getByTestId("component-brief-input").fill("主/次/幽灵按钮，含悬停与禁用态");
  await page.getByTestId("add-component-submit").click();
  await page.getByTestId("component-item-button-set").click();
  await page.getByTestId("component-generate").first().click();
  await page.locator('[data-testid^="component-candidate-"]').first().waitFor();
  await page.locator('[data-testid^="component-candidate-"] [data-testid="action-pick"]').first().click();
  await page.locator('[data-testid$="picked-badge"]').first().waitFor();
  await page.waitForTimeout(300);
  await shot(page, "07-component-picked");
  log("07 component generated + picked");

  // 暗色 + 英文
  await page.getByTestId("theme-toggle").click();
  await page.waitForTimeout(200);
  await shot(page, "08-dark");
  const en = page.getByRole("button", { name: /EN|English/i }).first();
  if (await en.count()) { await en.click(); } else {
    await page.locator("select").first().selectOption({ index: 1 }).catch(() => log("lang switch: manual"));
  }
  await page.waitForTimeout(200);
  await shot(page, "09-dark-en");
  log("08-09 dark + en done");

  await browser.close();
  console.log("[walk] PASS: 9 screenshots in", OUT);
};

run().catch((e) => { console.error("[walk] FAIL:", e.message); process.exit(1); });
