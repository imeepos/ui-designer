// 冒烟：全屏无限画布预览（lightbox 重做）— 全屏铺满 / 滚轮缩放 / 拖拽平移 / 1:1 / Esc 关闭
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

  await page.goto(BASE, { waitUntil: "networkidle" });
  await page.getByTestId("new-project").click();
  await page.getByTestId("project-name-input").fill("画布预览冒烟");
  await page.getByTestId("create-project-submit").click();
  await page.getByTestId("board-section").waitFor();
  await page.getByTestId("board-brand-input").fill("远洋航运, 深海军蓝, 黄铜");
  await page.getByTestId("board-generate").click();
  await page.locator('[data-testid^="board-candidate-"]').first().waitFor();

  // 打开放大预览
  await page.locator('[data-testid^="board-candidate-"] [data-testid="action-enlarge"]').first().click();
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
