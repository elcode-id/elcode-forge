import { test, expect } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

test.describe("Dashboard E2E Smoke Suite", () => {
  test("loads fixture bundle and renders all 7 dashboard screens", async ({ page }) => {
    page.on("console", (msg) => console.log("PAGE LOG:", msg.type(), msg.text()));
    page.on("pageerror", (err) => console.log("PAGE ERROR:", err.message, err.stack));

    // Navigate to root
    await page.goto("/");
    await expect(page).toHaveTitle(/ELCODE Forge/);

    // Upload the next-app fixture bundle
    const fixturePath = path.resolve(__dirname, "fixtures", "next-app.bundle.json");
    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(fixturePath);

    // 1. Overview Screen
    await expect(page.locator("text=Overall Health")).toBeVisible();
    await expect(page.locator("text=Health Dimensions Breakdown")).toBeVisible();
    await expect(page.locator("text=next-app.bundle.json")).toBeVisible();

    // 2. Architecture Screen
    await page.click('a[href="/architecture"]');
    await expect(page).toHaveURL(/.*\/architecture/);
    await expect(page.locator("text=Architecture & Dependency Graph")).toBeVisible();
    // Switch to accessible list view to verify module table
    await page.click("text=Module List (Accessible)");
    await expect(page.locator("table[aria-label='Modules list']")).toBeVisible();

    // 3. Findings Screen
    await page.click('a[href="/findings"]');
    await expect(page).toHaveURL(/.*\/findings/);
    await expect(page.locator("text=Findings & Diagnostics")).toBeVisible();
    await expect(page.locator("text=arch/no-direct-db-in-ui").first()).toBeVisible();

    // 4. Security Screen
    await page.click('a[href="/security"]');
    await expect(page).toHaveURL(/.*\/security/);
    await expect(page.locator("text=Security Intelligence")).toBeVisible();
    await expect(page.locator("text=Total Security Findings")).toBeVisible();

    // 5. Dependencies Screen
    await page.click('a[href="/dependencies"]');
    await expect(page).toHaveURL(/.*\/dependencies/);
    await expect(page.locator("text=Package Dependencies & Health")).toBeVisible();
    await expect(page.locator("table[aria-label='Dependencies table']")).toBeVisible();

    // 6. Changes Screen
    await page.click('a[href="/changes"]');
    await expect(page).toHaveURL(/.*\/changes/);
    await expect(page.locator("text=Changes & Blast Radius")).toBeVisible();

    // 7. Trends Screen (in Static File Mode)
    await page.click('a[href="/trends"]');
    await expect(page).toHaveURL(/.*\/trends/);
    await expect(page.locator("text=Trends & History Tracking")).toBeVisible();
    await expect(page.locator("text=Historical Trends Requires Live Mode")).toBeVisible();
  });
});
