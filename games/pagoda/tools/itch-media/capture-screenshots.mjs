#!/usr/bin/env node
// Captures the itch.io screenshots of Pagoda from the real web build.
//
// Serves dist/web/ on 127.0.0.1, drives a headless Chrome over the DevTools
// protocol, and lets a small in-page bot play: each frame it reads one pixel
// row through the slide and one through the top tile (WebGL readback with
// preserveDrawingBuffer), predicts where the slide will be next frame and
// presses Space (straight into miniquad's wasm exports) when that lands on the
// top tile's centre. An `offset` makes it miss on purpose (slices, game over).
//
// Requires: Node 22+ (global fetch/WebSocket), Google Chrome, and a fresh
// bundle from ./tools/build-web.sh.
// Output:   dist/itch-media/screenshots/*.png (1200×900, the game's 4:3)
//
// Usage:    node tools/itch-media/capture-screenshots.mjs

import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, existsSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const CRATE = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const WEB = join(CRATE, "dist/web");
const OUT = join(CRATE, "dist/itch-media/screenshots");
const CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";
const W = 1200, H = 900; // 800×600 virtual × 1.5, no letterbox
const HTTP_PORT = 8765, CDP_PORT = 9334;

if (!existsSync(join(WEB, "index.html"))) {
  console.error(`No web bundle in ${WEB}. Run ./tools/build-web.sh first.`);
  process.exit(1);
}
mkdirSync(OUT, { recursive: true });

// ─── Static server for dist/web ───
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm",
  ".wav": "audio/wav", ".ttf": "font/ttf", ".otf": "font/otf", ".png": "image/png" };
const server = createServer((req, res) => {
  const path = join(WEB, decodeURIComponent(req.url.split("?")[0]).replace(/\/$/, "/index.html"));
  if (!path.startsWith(WEB) || !existsSync(path)) { res.writeHead(404).end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" });
  res.end(readFileSync(path));
}).listen(HTTP_PORT, "127.0.0.1");

// ─── In-page bot (injected before the game creates its WebGL context) ───
const BOT = `(() => {
  const orig = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (t, a) {
    return orig.call(this, t, Object.assign({}, a || {}, { preserveDrawingBuffer: true }));
  };
  // slideY / topY: screen rows through the sliding tile and the top tile once
  // the camera has settled (virtual 400 / 440 × 1.5).
  const B = window.__bot = { on: false, last: 0, presses: 0, slideY: 600, topY: 660, offset: 0 };
  B.press = () => {
    wasm_exports.key_down(32, 0, false); wasm_exports.key_press(32);
    setTimeout(() => wasm_exports.key_up(32, 0), 30);
    B.presses++; B.last = performance.now();
  };
  const row = new Uint8Array(4 * 8192);
  function centre(y) { // centre of the widest bright run on screen row y
    const w = gl.drawingBufferWidth, h = gl.drawingBufferHeight;
    gl.readPixels(0, h - 1 - y, w, 1, gl.RGBA, gl.UNSIGNED_BYTE, row);
    let best = 0, bx = 0, run = 0, start = 0;
    for (let x = 0; x <= w; x++) {
      const o = 4 * x, on = x < w && Math.max(row[o], row[o + 1], row[o + 2]) > 95;
      if (on) { if (!run) start = x; run++; } else { if (run > best) { best = run; bx = start; } run = 0; }
    }
    return best > 40 ? bx + best / 2 : null;
  }
  let prev = null;
  function tick(now) {
    requestAnimationFrame(tick);
    if (!B.on || typeof gl === "undefined") return;
    const c = centre(B.slideY), t = centre(B.topY);
    if (c === null || t === null) { prev = null; return; }
    if (now - B.last < 250) { prev = c; return; }
    if (prev !== null) {
      const d = c - prev; // px per frame
      if (Math.abs(d) > 1 && Math.abs(d) < 40 && Math.abs(c + d - (t + B.offset)) <= Math.abs(d) / 2 + 0.5) {
        B.press(); prev = null; return;
      }
    }
    prev = c;
  }
  requestAnimationFrame(tick);
})();`;

// ─── Chrome + CDP ───
const chrome = spawn(CHROME, ["--headless=new", `--remote-debugging-port=${CDP_PORT}`,
  `--user-data-dir=${mkdtempSync(join(tmpdir(), "pagoda-shots-"))}`, `--window-size=${W},${H}`,
  "--hide-scrollbars", "--use-angle=swiftshader", "--enable-unsafe-swiftshader", "about:blank"],
  { stdio: "ignore" });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let targets;
for (let i = 0; i < 50 && !targets; i++) {
  try { targets = await (await fetch(`http://127.0.0.1:${CDP_PORT}/json`)).json(); } catch { await sleep(200); }
}
const ws = new WebSocket(targets.find((t) => t.type === "page").webSocketDebuggerUrl);
await new Promise((r) => (ws.onopen = r));
let nextId = 0;
const pending = new Map();
ws.onmessage = (e) => {
  const m = JSON.parse(e.data);
  if (m.id && pending.has(m.id)) { pending.get(m.id)(m.result); pending.delete(m.id); }
};
const send = (method, params = {}) => new Promise((r) => {
  const id = ++nextId; pending.set(id, r); ws.send(JSON.stringify({ id, method, params }));
});
const js = async (expr) => (await send("Runtime.evaluate", { expression: expr, returnByValue: true })).result?.value;
const shot = async (name) => {
  const { data } = await send("Page.captureScreenshot", { format: "png" });
  writeFileSync(join(OUT, `${name}.png`), Buffer.from(data, "base64"));
  console.log(`  ✓ ${name}.png`);
};
const untilPresses = async (n) => {
  const t0 = Date.now();
  for (;;) {
    const presses = await js("window.__bot ? __bot.presses : -1");
    if (presses < 0) throw new Error("bot not injected");
    if (presses >= n) break;
    if (Date.now() - t0 > 90_000) throw new Error(`bot stuck below ${n} drops`);
    await sleep(15);
  }
};

try {
  await send("Page.enable");
  await send("Runtime.enable");
  await send("Emulation.setDeviceMetricsOverride", { width: W, height: H, deviceScaleFactor: 1, mobile: false });
  await send("Page.addScriptToEvaluateOnNewDocument", { source: BOT });
  await send("Page.navigate", { url: `http://127.0.0.1:${HTTP_PORT}/` });
  // Wait for the wasm module and the WebGL context, then for the first frames.
  for (let i = 0; i < 100 && !(await js("typeof wasm_exports !== 'undefined' && typeof gl !== 'undefined'")); i++) {
    await sleep(100);
  }
  await sleep(1500);
  console.log(`→ Capturing into ${OUT}`);

  await shot("01-menu");

  await js("__bot.press()"); // start the run
  await sleep(300);
  await js("__bot.on = true"); // then let the bot play
  await untilPresses(1 + 9);
  await sleep(110);
  await shot("02-perfect-combo"); // shockwave + floating PERFECT ×N

  await untilPresses(1 + 30);
  await sleep(330);
  await shot("03-high-tower"); // cinnabar tiers, slide approaching

  await js("__bot.offset = 60"); // land off-centre: the overhang is sliced
  await untilPresses(1 + 33);
  await sleep(140);
  await shot("04-slice");

  await js("__bot.offset = 400"); // too far: a clean miss ends the run
  await untilPresses(1 + 34);
  await js("__bot.on = false");
  await sleep(1500);
  await shot("05-game-over");
} finally {
  ws.close();
  chrome.kill();
  server.close();
}
