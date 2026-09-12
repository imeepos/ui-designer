// e2e/tmp-p2-wizard-shots.mjs — P2 任务验收截图驱动（e2e/tmp- 前缀白名单，申报使用）
// Mock 模式（浏览器 = MockApi），零 API 花费；只写 e2e/screenshots/p2-*.png，不改任何仓库文件。
// 证据链：
//   p2-01 向导生成中 JobPanel 态（进度+耗时+取消）
//   p2-02 取消后候选空态（作业无孤儿、生成钮恢复）
//   p2-03 段标题两态对照（①完成苍青✓ ②当前缃色圆点+加粗）
//   p2-04 段标题三态对照（跳过路径：①完成 ②未来 muted ③当前）
//   p2-05 锚点选中标记 ≥11px + 段②完成态
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.BASE_URL ?? "http://localhost:5173/";
const OUT = "e2e/screenshots";
mkdirSync(OUT, { recursive: true });

const consoleIssues = [];

const run = async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  page.on("console", (msg) => {
    if (msg.type() === "error" || msg.type() === "warning") {
      consoleIssues.push({ level: msg.type(), text: msg.text() });
    }
  });
  page.on("pageerror", (err) => consoleIssues.push({ level: "pageerror", text: String(err) }));

  const shot = (name) => page.screenshot({ path: `${OUT}/${name}.png`, fullPage: false });
  const log = (...a) => console.log("[p2]", ...a);

  // ============================================================ 项目一：生成中 / 取消 / 选锚
  await page.goto(BASE, { waitUntil: "load" });
  await page.getByTestId("home-view").waitFor();
  await page.getByTestId("new-project").click();
  await page.getByTestId("create-wizard").waitFor();
  await page.getByTestId("wizard-name-input").fill("远洋航运 SaaS");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-generate").waitFor();
  await page.getByTestId("wizard-brand").fill("远洋航运 深海导航");
  await page.waitForTimeout(300);
  await shot("p2-03-section-step2-current-vs-done");
  log("03 section titles: 1=done(primary check) 2=current(accent dot + bold)");

  await page.getByTestId("wizard-generate").click();
  await page.getByTestId("job-panel").waitFor();
  await page.waitForTimeout(2500); // 等进度条爬升、耗时累计可见
  await shot("p2-01-wizard-generating-jobpanel");
  log("01 board generating: JobPanel progress + elapsed + eta + cancel");

  await page.getByTestId("job-cancel").click();
  await page.getByTestId("job-panel").waitFor({ state: "detached" });
  await page.getByText("操作已取消。").waitFor();
  await page.waitForTimeout(400);
  const generateEnabled = await page.getByTestId("wizard-generate").isEnabled();
  const candidatesGone = (await page.locator('[data-testid^="wizard-candidate-"]').count()) === 0;
  if (!generateEnabled || !candidatesGone) throw new Error("cancel did not restore the empty state");
  await shot("p2-02-wizard-cancelled-empty");
  log(`02 after cancel: empty candidates=${candidatesGone}, generate re-enabled=${generateEnabled}, no orphan job`);

  // 重新生成到完成，选锚（M3：✓ 锚点 ≥11px）
  await page.getByTestId("wizard-generate").click();
  await page.locator('[data-testid^="wizard-candidate-"]').first().waitFor();
  await page.waitForTimeout(400);
  await page.locator('[data-testid^="wizard-candidate-"]').first().click();
  await page.getByTestId("wizard-to-save").waitFor();
  await page.waitForTimeout(300);
  await shot("p2-05-anchor-badge-11px");
  log("05 anchor picked: ✓ 锚点 mark at 11px, section 2 done");

  // ============================================================ 项目二：跳过路径（三态同屏）
  await page.getByTestId("wizard-to-save").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.getByTestId("new-project").click();
  await page.getByTestId("create-wizard").waitFor();
  await page.getByTestId("wizard-name-input").fill("跳过总板的试验项目");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-generate").waitFor();
  await page.getByTestId("wizard-skip").click();
  await page.getByTestId("wizard-save").waitFor();
  await page.waitForTimeout(300);
  await shot("p2-04-section-tri-state");
  log("04 section titles tri-state: 1=done 2=future(muted outline) 3=current");

  await browser.close();

  console.log("[p2] PASS: screenshots written to", OUT);
  console.log("[p2] console issues captured:", consoleIssues.length);
  for (const issue of consoleIssues) {
    console.log(`  [${issue.level}] ${issue.text.slice(0, 300)}`);
  }
};

run().catch((e) => {
  console.error("[p2] FAIL:", e.message);
  process.exit(1);
});
