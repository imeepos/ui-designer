// 冒烟：全屏无限画布预览（lightbox）— 全屏铺满 / 滚轮缩放 / 拖拽平移 / 1:1 / Esc 关闭
// 入口：工作区 → 切换设计稿弹框 → 候选卡「放大」。
import { chromium } from "playwright";
import { mkdirSync } from "node:fs";

const BASE = process.env.BASE_URL ?? "http://localhost:4173/";
const OUT = "e2e/screenshots";
mkdirSync(OUT, { recursive: true });

const run = async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.setDefaultTimeout(15000);
  const log = (...a) => console.log("[lightbox]", ...a);

  const waitJobDone = async () => {
    await page.getByTestId("job-panel").waitFor();
    await page.getByTestId("job-panel").waitFor({ state: "detached" });
  };

  await page.goto(BASE, { waitUntil: "networkidle" });
  // 向导建项目并生成总览候选（锚点不设也行，页面候选即可放大预览）
  await page.getByTestId("new-project").click();
  await page.getByTestId("create-wizard").waitFor();
  await page.getByTestId("wizard-name-input").fill("画布预览冒烟");
  await page.getByTestId("wizard-brief-input").fill("墨青色系，克制专业的工具感");
  await page.getByTestId("wizard-create").click();
  await page.getByTestId("wizard-generate").waitFor();
  await page.getByTestId("wizard-generate").click();
  await page.getByTestId("wizard-candidates").waitFor();
  await page.locator('[data-testid^="wizard-candidate-"]').first().waitFor();
  await page.locator('[data-testid^="wizard-candidate-"]').first().click();
  await page.getByTestId("wizard-to-save").waitFor();
  await page.getByTestId("wizard-to-save").click();
  await page.getByTestId("wizard-save").click();
  await page.getByTestId("workspace-view").waitFor();
  log("project created; overview candidates ready");

  // 页面生成一批候选 → 打开切换弹框 → 候选卡「放大」进入 lightbox
  await page.getByTestId("menu-add-pages").click();
  await page.getByTestId("add-page-dialog").waitFor();
  await page.getByTestId("page-slug-input").fill("preview");
  await page.locator('[data-testid="add-page-form"] textarea').first().fill("画布预览用布局简报");
  await page.getByTestId("add-page-submit").click();
  await page.getByTestId("menu-page-preview").waitFor();
  await page.getByTestId("stage-regenerate").click();
  await page.getByTestId("regen-drawer").waitFor();
  await page.getByTestId("drawer-generate").click();
  await waitJobDone();
  await page.getByTestId("drawer-close").click();
  await page.getByTestId("stage-empty-switch").click();
  await page.getByTestId("switch-dialog").waitFor();
  const firstCard = page.locator('[data-testid^="switch-candidate-"]').first();
  await firstCard.waitFor();
  await firstCard.hover();
  await page.locator('[data-testid^="switch-preview-"]').first().click();
  await page.getByTestId("lightbox").waitFor();

  // 1) 全屏铺满：lightbox 根节点覆盖整个视口
  const root = await page.getByTestId("lightbox").boundingBox();
  const vp = page.viewportSize();
  if (!root || root.width < vp.width || root.height < vp.height) {
    throw new Error(`lightbox not fullscreen: ${JSON.stringify(root)}`);
  }
  const readZoom = () => page.getByTestId("lightbox-zoom-level").textContent();
  const initialZoom = await readZoom();
  log("open: fullscreen ok, initial zoom =", initialZoom);
  await page.screenshot({ path: `${OUT}/lb-01-open.png` });

  // 2) 滚轮缩放（滚轮向上放大），百分比随动
  await page.getByTestId("lightbox-canvas").hover({ position: { x: 720, y: 450 } });
  await page.mouse.wheel(0, -480);
  await page.waitForTimeout(200);
  const zoomed = await readZoom();
  if (zoomed === initialZoom) throw new Error(`wheel zoom no-op: ${initialZoom} -> ${zoomed}`);
  log("wheel zoom:", initialZoom, "->", zoomed);

  // 3) 拖拽平移：图片层 transform 位移变化
  const tx = () =>
    page.evaluate(() => {
      const layer = document.querySelector('[data-testid="lightbox-image"]')?.parentElement;
      return layer ? new DOMMatrixReadOnly(getComputedStyle(layer).transform).m41 : NaN;
    });
  const before = await tx();
  await page.mouse.move(720, 450);
  await page.mouse.down();
  await page.mouse.move(900, 520, { steps: 8 });
  await page.mouse.up();
  await page.waitForTimeout(120);
  const after = await tx();
  if (Number.isNaN(before) || Number.isNaN(after) || Math.abs(after - before) < 10) {
    throw new Error(`pan no-op: ${before} -> ${after}`);
  }
  log(`pan: x ${before.toFixed(0)}px -> ${after.toFixed(0)}px`);
  await page.screenshot({ path: `${OUT}/lb-02-zoom-pan.png` });

  // 4) 1:1 实际大小 → 100%
  await page.getByTestId("lightbox-actual").click();
  await page.waitForTimeout(120);
  if ((await readZoom()) !== "100%") throw new Error(`1:1 zoom = ${await readZoom()}`);
  const px = await page.evaluate(
    () => document.querySelector('[data-testid="lightbox-image"]')?.getBoundingClientRect().width,
  );
  log("1:1 ok, bitmap box width =", px, "px (natural pixels)");
  await page.screenshot({ path: `${OUT}/lb-03-actual.png` });

  // 5) 适应窗口 + Esc 关闭
  await page.getByTestId("lightbox-fit").click();
  await page.waitForTimeout(120);
  await page.keyboard.press("Escape");
  await page.waitForTimeout(120);
  if (await page.getByTestId("lightbox").isVisible().catch(() => false)) {
    throw new Error("Esc did not close lightbox");
  }
  log("fit + Esc close ok");
  await browser.close();
  console.log("[lightbox] PASS");
};

run().catch((error) => {
  console.error("[lightbox] FAIL:", error);
  process.exit(1);
});
