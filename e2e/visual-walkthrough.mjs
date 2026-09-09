// 视觉走查（增量规格）：首页项目列表 → 新建向导（信息/生成总览/保存）→ 工作区
// （左菜单+中央大图）→ 抽屉重生成 → 弹框切换设计稿 → 血缘面板 → 暗色/英文 → 无锚门控 toast。
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

  const waitJobDone = async () => {
    await page.getByTestId("job-panel").waitFor();
    await page.getByTestId("job-panel").waitFor({ state: "detached" });
  };

  await page.goto(BASE, { waitUntil: "networkidle" });
  await page.getByTestId("home-view").waitFor();
  await shot(page, "01-home-empty");
  log("01 home empty ok, api-mode:", await page.getByTestId("api-mode").textContent().catch(() => "n/a"));

  // 向导 ①：项目信息 → 创建
  await page.getByTestId("new-project").click();
  await page.getByTestId("create-wizard").waitFor();
  await page.getByTestId("wizard-name-input").fill("远洋航运 SaaS");
  await page.getByTestId("wizard-brief-input").fill("深海航行工作室气质，海军蓝主色+琥珀金点缀，克制专业");
  await shot(page, "02-wizard-info");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-generate").waitFor();
  log("02 wizard step1 created, step2 unlocked");

  // 向导 ②：生成总览 + 选锚
  await page.getByTestId("wizard-generate").click();
  await page.getByTestId("wizard-candidates").waitFor();
  await page.locator('[data-testid^="wizard-candidate-"]').first().waitFor();
  await shot(page, "03-wizard-overview-candidates");
  await page.locator('[data-testid^="wizard-candidate-"]').first().click();
  await page.locator('[data-testid^="wizard-candidate-"] >> nth=0').locator("svg").first().waitFor();
  await shot(page, "04-wizard-anchor-picked");
  log("03-04 overview generated + anchor picked inside wizard");

  // 向导 ③：保存进入项目 → 工作区总览大图
  await page.getByTestId("wizard-to-save").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("stage-image").waitFor();
  await shot(page, "05-workspace-overview");
  log("05 entered workspace; anchor on stage");

  // 左菜单添加页面 → 舞台空态
  await page.getByTestId("menu-add-pages").click();
  await page.getByTestId("add-page-dialog").waitFor();
  await page.getByTestId("page-slug-input").fill("dashboard");
  await page.locator('[data-testid="add-page-form"] textarea').first().fill("顶部4张指标卡，中部折线图，右侧任务列表");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("menu-page-dashboard").waitFor();
  await page.getByTestId("stage-empty-regenerate").waitFor();
  await shot(page, "06-page-added-empty-stage");
  log("06 page added via menu; stage shows empty state");

  // 抽屉重生成：编辑简报 → 生成（先 update 落库）→ 抽屉内进度
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("regen-drawer").waitFor();
  await page.getByTestId("drawer-brief").fill("顶部4张指标卡，中部折线图，右侧任务列表；底部加时间线");
  await page.getByTestId("drawer-generate").click();
  await page.getByTestId("job-panel").waitFor();
  await shot(page, "07-drawer-regenerating");
  await waitJobDone();
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").waitFor();
  log("07 drawer regenerated (brief persisted via update); candidates ready");

  // 弹框切换设计稿：预览+单选+确认 → 中央即时换图
  await page.getByTestId("stage-empty-switch").click();
  await page.getByTestId("switch-dialog").waitFor();
  await page.locator('[data-testid^="switch-candidate-"]').first().waitFor();
  await shot(page, "08-switch-dialog");
  await page.locator('[data-testid^="switch-candidate-"]').first().click();
  await page.getByTestId("switch-confirm").click();
  await page.getByTestId("stage-image").waitFor();
  await shot(page, "09-draft-switched");
  log("08-09 switch dialog picked; stage shows new current");

  // 血缘面板：打开 → 提示词/参数/徽标断言 → 截图 → 关闭
  await page.getByTestId("stage-lineage").click();
  await page.getByTestId("lineage-panel").waitFor();
  await page.getByTestId("lineage-prompt").waitFor();
  const promptText = await page.getByTestId("lineage-prompt").textContent();
  if (!promptText.includes("Image 1")) throw new Error(`lineage prompt missing board anchor line: ${promptText}`);
  await page.getByTestId("lineage-params").waitFor();
  await page.getByTestId("lineage-source-engine").waitFor();
  await shot(page, "10-lineage-panel");
  await page.getByTestId("lineage-close").click();
  await page.getByTestId("lineage-panel").waitFor({ state: "detached" });
  log("10 lineage panel: prompt + params + engine badge visible");

  // 组件同路径（压缩：添加→抽屉生成→切换）
  await page.getByTestId("menu-add-components").click();
  await page.getByTestId("add-component-dialog").waitFor();
  await page.getByTestId("component-name-input").fill("button-set");
  await page.getByTestId("component-brief-input").fill("主/次/幽灵按钮，含悬停与禁用态");
  await page.getByTestId("add-component-submit").click();
  await page.getByTestId("menu-component-button-set").waitFor();
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("drawer-generate").click();
  await waitJobDone();
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").click();
  await page.locator('[data-testid^="switch-candidate-"]').first().click();
  await page.getByTestId("switch-confirm").click();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(300);
  await shot(page, "11-component-picked");
  log("11 component flow done via drawer + dialog");

  // 暗色 + 英文
  await page.getByTestId("theme-toggle").click();
  await page.waitForTimeout(200);
  await shot(page, "12-dark");
  await page.getByTestId("language-switcher").getByRole("button", { name: "EN" }).click();
  await page.waitForTimeout(200);
  await shot(page, "13-dark-en");
  log("12-13 dark + en done");

  // 门控负例：向导跳过总览 → 工作区添加页面 → ANCHOR_REQUIRED 引导
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.getByTestId("new-project").click();
  await page.getByTestId("wizard-name-input").fill("尚未设锚的试验项目");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-skip").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("menu-add-pages").click();
  await page.getByTestId("toast-error").waitFor();
  const toastText = await page.getByTestId("toast-error").textContent();
  if (!toastText.includes("ANCHOR_REQUIRED")) throw new Error(`locked toast missing code: ${toastText}`);
  await shot(page, "14-gating-toast");
  log("14 gating: no-anchor add guides with ANCHOR_REQUIRED toast");

  await browser.close();
  console.log("[walk] PASS: 14 screenshots in", OUT);
};

run().catch((e) => { console.error("[walk] FAIL:", e.message); process.exit(1); });
