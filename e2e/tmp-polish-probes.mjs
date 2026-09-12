// e2e/tmp-polish-probes.mjs — P1 走查补充探针（e2e/tmp- 白名单）
// 1) 错误 toast 是否按 9s 自动消失；2) slug/组件简报行内错误；3) 左菜单未选中行悬停；
// 4) checking 瞬态（延迟 auth 桩）；5) 血缘参数 seed 显示是否截断（DOM 实测）。
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.BASE_URL ?? "http://localhost:5173/";
const OUT = "e2e/screenshots";
mkdirSync(OUT, { recursive: true });

const run = async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  await context.route("**/src/lib/api/auth.ts*", async (route) => {
    const original = await route.fetch();
    let body = await original.text();
    const needle = "if (!hasTauriRuntime()) return { loggedIn: false, account: null };";
    const patch = `if (!hasTauriRuntime()) {
    if (typeof window !== "undefined" && window.__rudderAuthStatus) {
      try { return await window.__rudderAuthStatus(); }
      catch (stubError) {
        throw new ApiError(
          stubError && stubError.__code ? stubError.__code : "UNKNOWN",
          stubError && stubError.message ? stubError.message : "auth stub failure",
          stubError && stubError.hint ? stubError.hint : undefined,
        );
      }
    }
    return { loggedIn: false, account: null };
  }`;
    if (body.includes(needle)) body = body.replace(needle, patch);
    await route.fulfill({ response: original, body, headers: original.headers() });
  });

  const page = await context.newPage();
  page.setDefaultTimeout(20000);
  const log = (...a) => console.log("[probe]", ...a);
  await page.addInitScript(() => { window.__rudderAuthStatus = null; });

  await page.goto(BASE, { waitUntil: "load" });
  await page.getByTestId("home-view").waitFor();

  // —— 建项目进工作区（跳过总板更快：直接建项目，不进 workspace 也行；
  // slug/组件错误需要已解锁菜单 → 需要锚点。走完整向导太重，这里用最短路径：
  // 向导内没有 slug 表单；必须在 workspace。故生成 1 张候选并选锚（count=1 加速）。）
  await page.getByTestId("new-project").click();
  await page.getByTestId("wizard-name-input").fill("探针项目");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-brand").fill("探针");
  await page.getByTestId("wizard-count-1").click();
  await page.getByTestId("wizard-generate").click();
  await page.locator('[data-testid^="wizard-candidate-"]').first().waitFor();
  await page.locator('[data-testid^="wizard-candidate-"]').first().click();
  await page.getByTestId("wizard-to-save").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();

  // —— 2a) 添加页面：slug 非法行内错误
  await page.getByTestId("menu-add-pages").click();
  await page.getByTestId("add-page-dialog").waitFor();
  await page.getByTestId("page-slug-input").fill("大屏");
  await page.getByTestId("page-slug-input").blur();
  await page.waitForTimeout(300);
  const slugVal = await page.getByTestId("page-slug-input").inputValue();
  log("slug after blur:", JSON.stringify(slugVal));
  await page.getByTestId("add-page-submit").click();
  await page.waitForTimeout(300);
  const slugErr = page.getByTestId("slug-error");
  log("slug-error visible:", await slugErr.isVisible().catch(() => false));
  if (await slugErr.isVisible().catch(() => false)) {
    log("slug-error text:", await slugErr.textContent());
    await page.screenshot({ path: `${OUT}/polish-09b-add-page-slug-error.png` });
    log("shot polish-09b-add-page-slug-error");
  }
  await page.getByTestId("page-slug-input").fill("dashboard");
  await page.locator('[data-testid="add-page-form"] textarea').first().fill("探针布局简报");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("stage-empty-regenerate").waitFor();

  // —— 2b) 添加组件：简报为空行内错误（P2 复验：客户端字段级文案，无泛化 toast）
  await page.getByTestId("menu-add-components").click();
  await page.getByTestId("add-component-dialog").waitFor();
  await page.getByTestId("component-name-input").fill("probe-set");
  await page.getByTestId("add-component-submit").click();
  await page.waitForTimeout(300);
  const briefErr = page.getByTestId("brief-error");
  log("brief-error visible:", await briefErr.isVisible().catch(() => false));
  if (await briefErr.isVisible().catch(() => false)) {
    log("brief-error text:", await briefErr.textContent());
    await page.screenshot({ path: `${OUT}/polish-10b-add-component-brief-error.png` });
    log("shot polish-10b-add-component-brief-error");
  }
  // 泛化 VALIDATION_ERROR toast 是否出现（应为否）
  log("VALIDATION_ERROR toast visible:", await page.getByTestId("toast-error").isVisible().catch(() => false));
  await page.getByTestId("add-component-dialog").press("Escape");

  // —— 1) 错误 toast 自动消失计时：auth 错误走行内不走 toast；
  // 改用无锚门控路径（toast.error(ANCHOR_REQUIRED)）。新开第二个无锚项目计时。
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.getByTestId("new-project").click();
  await page.getByTestId("wizard-name-input").fill("无锚计时项目");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-skip").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("menu-add-pages").click();
  const t0 = Date.now();
  await page.getByTestId("toast-error").waitFor();
  log("error toast appeared at +", Date.now() - t0, "ms");
  // 等 12s 看是否自动消失（ERROR_TIMEOUT_MS = 9000）
  let gone = false;
  try {
    await page.getByTestId("toast-error").waitFor({ state: "detached", timeout: 12000 });
    gone = true;
  } catch { gone = false; }
  log("error toast auto-dismissed:", gone, "at +", Date.now() - t0, "ms");

  // —— 4) checking 瞬态：延迟 800ms 的 auth 桩（当前在无锚项目工作区，设置是全局的）
  await page.evaluate(() => {
    window.__rudderAuthStatus = () =>
      new Promise((resolve) => {
        setTimeout(() => resolve({
          loggedIn: true,
          account: { user: { id: 7, email: "probe@rudder.studio", name: "探针", disabled: false, created_at: 1735689600 }, balance: 12 },
        }), 800);
      });
  });
  await page.getByTestId("settings-open").click();
  await page.getByTestId("settings-account-checking").waitFor().catch(() => log("checking testid not found"));
  await page.screenshot({ path: `${OUT}/polish-23b-settings-checking.png` });
  log("shot polish-23b-settings-checking (delayed stub)");
  await page.getByTestId("settings-account-user").waitFor();
  await page.getByTestId("settings-dialog").press("Escape");
  await page.waitForTimeout(200);

  // —— 3) 左菜单未选中行悬停：回探针项目（卡片倒序第 2 张）再操作
  await page.getByTestId("back-home").click();
  await page.getByTestId("home-view").waitFor();
  await page.locator('[data-testid^="project-card-"]').nth(1).click();
  await page.getByTestId("workspace-view").waitFor();
  await page.getByTestId("menu-page-dashboard").click();
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("drawer-generate").click();
  await page.getByTestId("job-panel").waitFor({ state: "detached" });
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").click();
  await page.locator('[data-testid^="switch-candidate-"]').first().click();
  await page.getByTestId("switch-confirm").click();
  await page.getByTestId("stage-image").waitFor();
  await page.getByTestId("menu-page-dashboard").hover();
  await page.waitForTimeout(300);
  await page.screenshot({ path: `${OUT}/polish-08b-menu-hover.png` });
  log("shot polish-08b-menu-hover (non-selected row hover)");

  // —— 5) 血缘 seed 显示 DOM 实测
  await page.getByTestId("stage-lineage").click();
  await page.getByTestId("lineage-panel").waitFor();
  await page.getByTestId("lineage-params").waitFor();
  const seedRow = await page.evaluate(() => {
    const rows = Array.from(document.querySelectorAll('[data-testid="lineage-params"] > div'));
    const row = rows.find((el) => el.textContent && el.textContent.includes("种子"));
    if (!row) return null;
    const dd = row.querySelector("dd");
    return {
      text: dd ? dd.textContent : null,
      clientWidth: dd ? dd.clientWidth : null,
      scrollWidth: dd ? dd.scrollWidth : null,
      overflowStyle: dd ? getComputedStyle(dd).textOverflow : null,
    };
  });
  log("lineage seed row:", JSON.stringify(seedRow));
  await page.screenshot({ path: `${OUT}/polish-17b-lineage-seed-row.png` });
  await page.getByTestId("lineage-close").click();

  await browser.close();
  console.log("[probe] DONE");
};

run().catch((e) => { console.error("[probe] FAIL:", e.message); process.exit(1); });
