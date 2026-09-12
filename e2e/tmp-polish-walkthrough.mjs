// e2e/tmp-polish-walkthrough.mjs — P1 走查评审专用临时驱动（e2e/tmp- 前缀白名单）
// 只读评审：不修改任何仓库文件；仅新增 e2e/screenshots/polish-*.png 截图。
// Mock 模式（浏览器 = MockApi），零 API 花费。
//
// 唯一的运行时干预：通过 Vite dev 模块拦截，给 src/lib/api/auth.ts 的
// fetchAuthStatus 浏览器降级分支注入 window.__rudderAuthStatus 桩，
// 以驱动设置对话框账号区的 signedIn / expired 两个纯前端状态
// （浏览器 Mock 下这两个状态原生不可达，见 findings 盲区节）。
// 其余一切界面行为均为真实 Mock 交互。
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.BASE_URL ?? "http://localhost:5173/";
const OUT = "e2e/screenshots";
mkdirSync(OUT, { recursive: true });

const consoleIssues = [];

const run = async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });

  // --- console / pageerror 捕获（走查全程） ---
  for (const evt of ["pageerror"]) {
    context.on(evt, (err) => consoleIssues.push({ level: "pageerror", text: String(err) }));
  }

  // --- auth 模块桩（仅 fetchAuthStatus 的浏览器降级分支） ---
  await context.route("**/src/lib/api/auth.ts*", async (route) => {
    const original = await route.fetch();
    let body = await original.text();
    const needle = "if (!hasTauriRuntime()) return { loggedIn: false, account: null };";
    const patch = `if (!hasTauriRuntime()) {
    if (typeof window !== "undefined" && window.__rudderAuthStatus) {
      try {
        return await window.__rudderAuthStatus();
      } catch (stubError) {
        throw new ApiError(
          stubError && stubError.__code ? stubError.__code : "UNKNOWN",
          stubError && stubError.message ? stubError.message : "auth stub failure",
          stubError && stubError.hint ? stubError.hint : undefined,
        );
      }
    }
    return { loggedIn: false, account: null };
  }`;
    if (!body.includes(needle)) {
      consoleIssues.push({ level: "driver", text: "auth.ts needle not found — auth states will stay native (signedOut)" });
    } else {
      body = body.replace(needle, patch);
    }
    await route.fulfill({ response: original, body, headers: original.headers() });
  });

  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  page.on("console", (msg) => {
    if (msg.type() === "error" || msg.type() === "warning") {
      consoleIssues.push({ level: msg.type(), text: msg.text() });
    }
  });
  page.on("pageerror", (err) => consoleIssues.push({ level: "pageerror", text: String(err) }));

  const shot = (name) => page.screenshot({ path: `${OUT}/${name}.png`, fullPage: false });
  const log = (...a) => console.log("[polish]", ...a);

  await page.addInitScript(() => { window.__rudderAuthStatus = null; });

  // ============================================================ A. 中文·亮色主流程
  await page.goto(BASE, { waitUntil: "load" });
  await page.getByTestId("home-view").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-01-home-empty");
  log("01 home empty (boat empty state)");

  // 向导 ①：项目信息 + 尺寸校验错误态
  await page.getByTestId("new-project").click();
  await page.getByTestId("create-wizard").waitFor();
  await page.getByTestId("wizard-name-input").fill("远洋航运 SaaS");
  await page.getByTestId("wizard-brief-input").fill("深海航行工作室气质，苍青主色+缃色点缀，克制专业");
  await shot("polish-02-wizard-info");
  await page.getByTestId("wizard-preset-custom").click();
  await page.getByTestId("wizard-width").fill("500");
  await page.getByTestId("wizard-height").fill("500");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-size-error").waitFor();
  await shot("polish-03-wizard-size-error");
  log("02-03 wizard info + inline size error (detail-form error state)");
  await page.getByTestId("wizard-preset-web").click();

  // 向导 ②：生成总览（生成中 → 候选 → 选锚）
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-generate").waitFor();
  await page.getByTestId("wizard-brand").fill("远洋航运 深海导航");
  await page.getByTestId("wizard-color").fill("苍青为主，缃色点缀，宣纸留白");
  await page.getByTestId("wizard-generate").click();
  // 向导内生成态：spinner 文案行（无 job-panel / 无取消，见 findings）
  await page.getByText("正在生成总板候选…").waitFor();
  await shot("polish-04-wizard-generating");
  log("04 board generating (wizard spinner line, no cancel)");
  await page.locator('[data-testid^="wizard-candidate-"]').first().waitFor();
  await page.waitForTimeout(500);
  await shot("polish-05-wizard-candidates");
  await page.locator('[data-testid^="wizard-candidate-"]').first().click();
  await page.waitForTimeout(400);
  await shot("polish-06-wizard-anchor-picked");
  log("05-06 candidates + anchor picked");

  // 向导 ③：保存进入工作区
  await page.getByTestId("wizard-to-save").click();
  await page.waitForTimeout(200);
  await shot("polish-07-wizard-save");
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-08-workspace-overview");
  log("07-08 wizard save → workspace overview (anchor on canvas)");

  // 左菜单条目悬停（删除浮现）
  await page.getByTestId("menu-overview").hover();
  await page.waitForTimeout(250);
  await shot("polish-08b-menu-hover");

  // 详情表单：添加页面（slug 校验错误态）
  await page.getByTestId("menu-add-pages").click();
  await page.getByTestId("add-page-dialog").waitFor();
  await page.getByTestId("page-slug-input").fill("大屏");
  await page.locator('[data-testid="add-page-form"] textarea').first().click();
  await page.waitForTimeout(250);
  const slugError = page.getByTestId("slug-error");
  if (await slugError.isVisible().catch(() => false)) {
    await shot("polish-09b-add-page-slug-error");
    log("09b slug inline error visible");
  }
  await page.getByTestId("page-slug-input").fill("dashboard");
  await page.locator('[data-testid="add-page-form"] textarea').first().fill("顶部4张指标卡，中部折线图，右侧任务列表");
  await shot("polish-09-add-page-form");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("stage-empty-regenerate").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-11-stage-empty");
  log("09-11 add page form → stage empty state");

  // 重新生成抽屉（生成中）
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("regen-drawer").waitFor();
  await page.getByTestId("drawer-brief").fill("顶部4张指标卡，中部折线图，右侧任务列表；底部加时间线");
  await page.getByTestId("drawer-generate").click();
  await page.getByTestId("job-panel").waitFor();
  await shot("polish-12-drawer-generating");
  log("12 drawer generating (job panel)");
  await page.getByTestId("job-panel").waitFor({ state: "detached" });
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").waitFor();

  // 切换设计稿弹框 + 悬停放大 + 灯箱
  await page.getByTestId("stage-empty-switch").click();
  await page.getByTestId("switch-dialog").waitFor();
  await page.locator('[data-testid^="switch-candidate-"]').first().waitFor();
  await page.waitForTimeout(400);
  await shot("polish-13-switch-dialog");
  const firstCandidate = page.locator('[data-testid^="switch-candidate-"]').first();
  await firstCandidate.hover();
  await page.waitForTimeout(300);
  await shot("polish-14-switch-hover-actions");
  await page.locator('[data-testid^="switch-preview-"]').first().click();
  await page.getByTestId("lightbox").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-15-lightbox");
  await page.getByTestId("lightbox-close").click();
  await page.locator('[data-testid^="switch-candidate-"]').first().click();
  await page.getByTestId("switch-confirm").click();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-16-stage-page-picked");
  log("13-16 switch dialog + hover enlarge + lightbox + picked");

  // 血缘面板
  await page.getByTestId("stage-lineage").click();
  await page.getByTestId("lineage-panel").waitFor();
  await page.getByTestId("lineage-prompt").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-17-lineage-panel");
  await page.getByTestId("lineage-close").click();
  log("17 lineage panel");

  // 组件同路径（压缩）
  await page.getByTestId("menu-add-components").click();
  await page.getByTestId("add-component-dialog").waitFor();
  await page.getByTestId("component-name-input").fill("button-set");
  await page.getByTestId("component-brief-input").fill("主/次/幽灵按钮，含悬停与禁用态");
  await shot("polish-10-add-component-form");
  await page.getByTestId("add-component-submit").click();
  await page.getByTestId("menu-component-button-set").waitFor();
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("drawer-generate").click();
  await page.getByTestId("job-panel").waitFor({ state: "detached" });
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").click();
  await page.locator('[data-testid^="switch-candidate-"]').first().waitFor();
  await page.locator('[data-testid^="switch-candidate-"]').first().click();
  await page.getByTestId("switch-confirm").click();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-18-component-picked");
  log("10+18 component form + picked");

  // 导出对话框（表单 → 完成）
  await page.getByTestId("export-open").click();
  await page.getByTestId("export-dialog").waitFor();
  await page.waitForTimeout(300);
  await shot("polish-19-export-form");
  await page.getByTestId("export-start").click();
  await page.getByTestId("export-result").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-20-export-done");
  await page.getByTestId("export-done").click();
  log("19-20 export form + done");

  // 门控负例（ANCHOR_REQUIRED 错误 toast）
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
  await page.waitForTimeout(300);
  await shot("polish-21-gating-toast");
  log("21 gating toast (ANCHOR_REQUIRED error state)");

  // 首页卡片 + 悬停姿态
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.locator('[data-testid^="project-card-"]').first().waitFor();
  await page.waitForTimeout(400);
  await shot("polish-22-home-cards");
  await page.locator('[data-testid^="project-card-"]').nth(1).hover();
  await page.waitForTimeout(300);
  await shot("polish-22b-home-card-hover");
  log("22 home cards + hover posture");

  // ============================================================ B. 设置对话框（中文·亮色）
  const openSettings = async () => {
    await page.getByTestId("settings-open").click();
    await page.getByTestId("settings-dialog").waitFor();
  };
  const closeSettings = async () => {
    await page.getByTestId("settings-dialog").getByRole("button").first().click();
    await page.getByTestId("settings-dialog").waitFor({ state: "detached" });
    await page.waitForTimeout(200);
  };

  // ① checking 态（尽力抓拍，瞬态）
  await page.getByTestId("settings-open").click();
  await page.waitForTimeout(120);
  await shot("polish-23b-settings-checking").catch(() => {});
  await page.getByTestId("settings-dialog").waitFor();
  await page.getByTestId("settings-account-form").waitFor();
  await page.waitForTimeout(200);
  await shot("polish-23-settings-signedout");
  log("23 settings signedOut");
  await page.getByTestId("settings-dialog").press("Escape");
  await page.getByTestId("settings-dialog").waitFor({ state: "detached" });
  await page.waitForTimeout(200);

  // ② 行内必填校验错误（纯客户端）
  await openSettings();
  await page.getByTestId("settings-account-form").waitFor();
  await page.getByTestId("settings-email").fill("qingwu@rudder.studio");
  await page.getByTestId("settings-login").click();
  await page.getByTestId("settings-account-error").waitFor();
  await page.waitForTimeout(150);
  await shot("polish-24-settings-error-required");
  // ③ 登录降级错误（NOT_IMPLEMENTED，浏览器 Mock）
  await page.getByTestId("settings-password").fill("hunter2");
  await page.getByTestId("settings-login").click();
  await page.waitForTimeout(400);
  await shot("polish-25-settings-login-not-implemented");
  // ④ 连接测试失败态（NOT_IMPLEMENTED → destructive 面板）
  await page.getByTestId("settings-test").click();
  await page.getByTestId("settings-test-result").waitFor();
  await page.waitForTimeout(200);
  await shot("polish-26-settings-test-failed");
  log("24-26 settings inline errors + failed probe");
  await page.getByTestId("settings-dialog").press("Escape");
  await page.getByTestId("settings-dialog").waitFor({ state: "detached" });
  await page.waitForTimeout(200);

  // ⑤ 已登录卡（auth 桩）
  await page.evaluate(() => {
    window.__rudderAuthStatus = () => ({
      loggedIn: true,
      account: {
        user: { id: 42, email: "qingwu@rudder.studio", name: "沈青梧", disabled: false, created_at: 1735689600 },
        balance: 460,
      },
    });
  });
  await openSettings();
  await page.getByTestId("settings-account-user").waitFor();
  await page.waitForTimeout(300);
  await shot("polish-27-settings-signedin");
  log("27 settings signedIn (stubbed auth_status)");
  await page.getByTestId("settings-dialog").press("Escape");
  await page.getByTestId("settings-dialog").waitFor({ state: "detached" });
  await page.waitForTimeout(200);

  // ⑥ 会话过期引导（auth 桩抛 SESSION_EXPIRED）
  await page.evaluate(() => {
    window.__rudderAuthStatus = async () => {
      const err = new Error("stored cms cookie rejected");
      err.__code = "SESSION_EXPIRED";
      err.hint = "re-enter your password to continue";
      throw err;
    };
  });
  await openSettings();
  await page.getByTestId("settings-session-expired").waitFor();
  await page.waitForTimeout(200);
  await shot("polish-28-settings-expired");
  log("28 settings expired guidance");
  await page.getByTestId("settings-dialog").press("Escape");
  await page.getByTestId("settings-dialog").waitFor({ state: "detached" });
  await page.waitForTimeout(200);
  await page.evaluate(() => { window.__rudderAuthStatus = null; });

  // ============================================================ C. 暗色（中文）
  await page.getByTestId("theme-toggle").click();
  await page.waitForTimeout(300);
  await shot("polish-29-dark-home-cards");
  // 倒序列表第 2 张 = 已设锚项目（第 1 张是无锚试验项目）
  await page.locator('[data-testid^="project-card-"]').nth(1).click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-30-dark-workspace");
  await page.getByTestId("stage-switch").click();
  await page.getByTestId("switch-dialog").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-31-dark-switch-dialog");
  await page.getByTestId("switch-dialog").getByRole("button", { name: "取消" }).click();
  log("29-31 dark home + workspace + switch dialog");

  // ============================================================ D. 英文（回到亮色）
  await page.getByTestId("theme-toggle").click();
  await page.waitForTimeout(300);
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.getByTestId("language-switcher").getByRole("button", { name: "EN" }).click();
  await page.waitForTimeout(300);
  await shot("polish-32-en-home");
  await page.locator('[data-testid^="project-card-"]').nth(1).click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("stage-image").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-33-en-workspace");
  await page.getByTestId("settings-open").click();
  await page.getByTestId("settings-dialog").waitFor();
  await page.waitForTimeout(400);
  await shot("polish-34-en-settings");
  log("32-34 en home + workspace + settings");

  await browser.close();

  // ============================================================ 汇总
  console.log("[polish] PASS: screenshots written to", OUT);
  console.log("[polish] console issues captured:", consoleIssues.length);
  for (const issue of consoleIssues) {
    console.log(`  [${issue.level}] ${issue.text.slice(0, 300)}`);
  }
};

run().catch((e) => {
  console.error("[polish] FAIL:", e.message);
  process.exit(1);
});
