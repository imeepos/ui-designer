import { chromium } from "playwright";
const b = await chromium.launch();
const p = await b.newPage({ viewport: { width: 1440, height: 900 } });
await p.goto("http://localhost:4173/", { waitUntil: "networkidle" });
await p.locator('[data-testid^="settings"], button[title*="设置"], button[aria-label*="设置"]').first().click().catch(async () => {
  await p.getByRole("button").filter({ hasText: /⚙|settings/i }).first().click();
});
await p.waitForTimeout(500);
await p.screenshot({ path: "e2e/screenshots/16-settings-three-fields.png" });
const fields = await p.locator("input").evaluateAll(els => els.map(e => e.getAttribute("placeholder") || e.id || e.type));
console.log("inputs:", JSON.stringify(fields));
await b.close();
