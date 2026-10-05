// Optional creator-host browser proof. Dependencies stay outside the repository.
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.M30_PLAYWRIGHT_MODULE);
const origin = process.env.M30_INSPECTOR_URL || "http://127.0.0.1:41473";
assert.match(origin, /^http:\/\/127\.0\.0\.1:\d+$/);
const browser = await chromium.launch({
  headless: true,
  executablePath: process.env.M30_CHROMIUM_BIN,
  args: ["--no-sandbox", "--disable-background-networking"],
  timeout: 15000,
});
try {
  const page = await browser.newPage();
  const apiResponse = page.waitForResponse(response => response.url() === `${origin}/api/inspector/sessions`);
  await page.goto(origin, { waitUntil: "domcontentloaded", timeout: 15000 });
  const response = await apiResponse;
  assert.equal(response.status(), 200);
  const requestHeaders = await response.request().allHeaders();
  assert.equal(requestHeaders.origin, undefined);
  assert.equal(requestHeaders["sec-fetch-site"], "same-origin");
  assert.equal((await response.allHeaders())["cache-control"], "no-store");
  assert.ok(Array.isArray((await response.json()).sessions));
  const attacker = await browser.newPage();
  const foreignOrigin = "http://127.0.0.1:41474";
  await attacker.route(`${foreignOrigin}/`, route => route.fulfill({
    contentType: "text/html", body: "<!doctype html><title>Local origin fixture</title>",
  }));
  await attacker.goto(foreignOrigin);
  const readable = await attacker.evaluate(async target => {
    try { await (await fetch(`${target}/api/inspector/sessions`)).json(); return true; }
    catch { return false; }
  }, origin);
  assert.equal(readable, false);
  console.log(JSON.stringify({
    same_origin_browser_get: "pass",
    origin_header_absent_on_same_origin_get: true,
    no_store: "pass", foreign_browser_origin_cannot_read: "pass",
  }));
} finally {
  await browser.close();
}
