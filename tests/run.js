#!/usr/bin/env node
"use strict"

const fs = require("fs")
const path = require("path")
const vm = require("vm")
const assert = require("assert")

const ROOT = path.resolve(__dirname, "..")
const JS = path.join(ROOT, "js")
const FIX = path.join(__dirname, "fixtures")

function stripPragma(src) {
  return src
    .replace(/^\.pragma library\s*\n/, "")
    .replace(/^\.import\s+"[^"]+"\s+as\s+\w+\s*\n/gm, "")
}

function loadEngine(file, extra) {
  const src = stripPragma(fs.readFileSync(path.join(JS, file), "utf8"))
  const sandbox = Object.assign(
    {
      console,
      Date,
      Math,
      JSON,
      String,
      Number,
      Array,
      Object,
      parseInt,
      isNaN,
      exports: {},
      module: { exports: {} }
    },
    extra || {}
  )
  vm.createContext(sandbox)
  vm.runInContext(src, sandbox, { filename: file })
  const exported = {}
  for (const key of Object.keys(sandbox)) {
    if (
      [
        "console",
        "Date",
        "Math",
        "JSON",
        "String",
        "Number",
        "Array",
        "Object",
        "parseInt",
        "isNaN",
        "exports",
        "module",
        "Color"
      ].indexOf(key) >= 0
    )
      continue
    exported[key] = sandbox[key]
  }
  return exported
}

const Color = loadEngine("Color.js")
const Theme = loadEngine("Theme.js", { Color })
const History = loadEngine("History.js")
const Capture = loadEngine("Capture.js")
const ThemeSession = loadEngine("ThemeSession.js")

let passed = 0
let failed = 0

function test(name, fn) {
  try {
    History.reset()
    ThemeSession.reset()
    fn()
    passed += 1
    process.stdout.write("ok  " + name + "\n")
  } catch (err) {
    failed += 1
    process.stderr.write("FAIL " + name + "\n" + (err && err.stack ? err.stack : err) + "\n")
  }
}

function fixture(name) {
  return fs.readFileSync(path.join(FIX, name), "utf8")
}

function jsonFix(name) {
  return JSON.parse(fixture(name))
}

test("color: hex round-trip", () => {
  assert.strictEqual(Color.normalizeHex("#ABC"), "#aabbcc")
  assert.strictEqual(Color.rgbToHex(255, 0, 170), "#ff00aa")
  const rgb = Color.hexToRgb("#ff00aa")
  assert.strictEqual(rgb.r, 255)
  assert.strictEqual(rgb.g, 0)
  assert.strictEqual(rgb.b, 170)
})

test("color: hsl round-trip red", () => {
  const hsl = Color.rgbToHsl(255, 0, 0)
  assert.ok(Math.abs(hsl.h - 0) < 0.01)
  assert.ok(hsl.s > 99)
  const rgb = Color.hslToRgb(hsl.h, hsl.s, hsl.l)
  assert.strictEqual(rgb.r, 255)
  assert.strictEqual(rgb.g, 0)
  assert.strictEqual(rgb.b, 0)
})

test("color: oklch of sRGB red is chromatic", () => {
  const lch = Color.rgbToOklch(255, 0, 0)
  assert.ok(lch.C > 0.2)
  assert.ok(lch.L > 0.5 && lch.L < 0.7)
  const back = Color.oklchToRgb(lch.L, lch.C, lch.h)
  assert.ok(Math.abs(back.r - 255) <= 2)
  assert.ok(back.g <= 2)
  assert.ok(back.b <= 2)
})

test("color: WCAG black/white is 21:1", () => {
  const info = Color.contrastInfo("#000000", "#ffffff")
  assert.ok(Math.abs(info.ratio - 21) < 0.05)
  assert.strictEqual(info.rating, "AAA")
  assert.strictEqual(info.aa, true)
})

test("color: WCAG gray on white fails AA", () => {
  const info = Color.contrastInfo("#aaaaaa", "#ffffff")
  assert.ok(info.ratio < 4.5)
  assert.strictEqual(info.aa, false)
  assert.strictEqual(info.rating, "fail")
})

test("color: describe fills every space", () => {
  const d = Color.describe("#7aa2f7")
  assert.strictEqual(d.hex, "#7aa2f7")
  assert.ok(d.rgb.indexOf("rgb(") === 0)
  assert.ok(d.hsl.indexOf("hsl(") === 0)
  assert.ok(d.oklch.indexOf("oklch(") === 0)
})

test("color: lightenUntilContrast reaches AA", () => {
  const hex = Color.lightenUntilContrast("#444444", "#111111", 4.5)
  const info = Color.contrastInfo(hex, "#111111")
  assert.ok(info.ratio >= 4.5)
})

test("theme: darkest is background, highest chroma is accent", () => {
  const palette = jsonFix("palette-photo.json")
  const mapped = Theme.mapPalette(palette)
  assert.strictEqual(mapped.roles.background, "#1b1423")
  assert.strictEqual(mapped.roles.foreground.toLowerCase() !== "#1b1423", true)
  assert.ok(mapped.contrast.ratio >= 4.5)
  const accentC = Color.chromaOf(mapped.roles.accent)
  const bgC = Color.chromaOf(mapped.roles.background)
  assert.ok(accentC > bgC)
})

test("theme: generate writes every Omarchy colors.toml key", () => {
  const gen = Theme.generate(jsonFix("palette-photo.json"))
  const parsed = Theme.parseColorsToml(gen.files["colors.toml"])
  const check = Theme.validateTokens(parsed)
  assert.strictEqual(check.ok, true, check.error)
  assert.ok(gen.files["hyprland.conf"].indexOf("col.active_border") >= 0)
  assert.ok(gen.files["alacritty.toml"].indexOf("[colors.primary]") >= 0)
})

test("theme: tokyo-night fixture parses and validates", () => {
  const tokens = Theme.parseColorsToml(fixture("colors-tokyo-night.toml"))
  assert.strictEqual(tokens.background, "#1a1b26")
  assert.strictEqual(tokens.accent, "#7aa2f7")
  const check = Theme.validateTokens(tokens)
  assert.strictEqual(check.ok, true, check.error)
  assert.ok(check.contrast.ratio >= 4.5)
})

test("theme: catppuccin fixture parses and validates", () => {
  const tokens = Theme.parseColorsToml(fixture("colors-catppuccin.toml"))
  assert.strictEqual(tokens.background, "#1e1e2e")
  assert.strictEqual(tokens.accent, "#89b4fa")
  const check = Theme.validateTokens(tokens)
  assert.strictEqual(check.ok, true, check.error)
})

test("theme: mapping a tokyo-night-like palette keeps a dark bg and the highest-chroma accent", () => {
  const palette = ["#1a1b26", "#c0caf5", "#7aa2f7", "#f7768e", "#9ece6a", "#e0af68"]
  const mapped = Theme.mapPalette(palette)
  assert.strictEqual(mapped.roles.background, "#1a1b26")
  assert.ok(Color.chromaOf(mapped.roles.accent) >= Color.chromaOf("#7aa2f7") - 0.001)
  assert.ok(mapped.contrast.ratio >= 4.5)
  assert.strictEqual(Theme.validateTokens(mapped.tokens).ok, true)
})

test("theme session: snapshot on every preview while not live", () => {
  ThemeSession.beginPreview("tokyo-night", false)
  assert.strictEqual(ThemeSession.snapshot().original, "tokyo-night")
  ThemeSession.markApplied()
  ThemeSession.beginPreview("chroma-preview", true)
  assert.strictEqual(ThemeSession.snapshot().original, "tokyo-night")
  ThemeSession.afterSuccessfulRevert()
  assert.strictEqual(ThemeSession.snapshot().original, "")
  assert.strictEqual(ThemeSession.snapshot().live, false)
  ThemeSession.beginPreview("catppuccin", false)
  assert.strictEqual(ThemeSession.snapshot().original, "catppuccin")
})

test("theme session: never records chroma-preview as the revert target", () => {
  ThemeSession.beginPreview("chroma-preview", false)
  assert.strictEqual(ThemeSession.snapshot().original, "")
})

test("theme: mapping a catppuccin-like palette keeps mocha-dark background", () => {
  const palette = ["#1e1e2e", "#cdd6f4", "#89b4fa", "#f38ba8", "#a6e3a1", "#f9e2af"]
  const mapped = Theme.mapPalette(palette)
  assert.strictEqual(mapped.roles.background, "#1e1e2e")
  assert.ok(mapped.contrast.ratio >= 4.5)
})

test("history: newest first, unique, capped", () => {
  History.push({ hex: "#ff0000" }, 3)
  History.push({ hex: "#00ff00" }, 3)
  History.push({ hex: "#0000ff" }, 3)
  History.push({ hex: "#ff0000" }, 3)
  const snap = History.snapshot()
  assert.strictEqual(snap.picks.length, 3)
  assert.strictEqual(snap.picks[0].hex, "#ff0000")
  assert.strictEqual(snap.picks[1].hex, "#0000ff")
  History.push({ hex: "#ffffff" }, 3)
  assert.strictEqual(History.snapshot().picks.length, 3)
  assert.strictEqual(History.snapshot().picks[0].hex, "#ffffff")
})

test("history: serialize/load round-trip", () => {
  History.push({ hex: "#7aa2f7", rgb: "rgb(122, 162, 247)" }, 8)
  const raw = History.serialize()
  History.reset()
  History.load(raw)
  assert.strictEqual(History.snapshot().picks[0].hex, "#7aa2f7")
})

test("capture: hyprctl cursorpos text and json", () => {
  const a = Capture.parseCursorText("1234, 56")
  assert.strictEqual(a.x, 1234)
  assert.strictEqual(a.y, 56)
  const b = Capture.parseCursorText("{\"x\":10.5,\"y\":20}")
  assert.strictEqual(b.x, 10.5)
  assert.strictEqual(b.y, 20)
})

test("capture: 1x mapping region stays on output", () => {
  const mons = jsonFix("monitors-1x.json")
  const r = Capture.regionAround(10, 10, 128, mons)
  assert.strictEqual(r.x, 0)
  assert.strictEqual(r.y, 0)
  assert.strictEqual(r.w, 128)
  const mid = Capture.regionAround(960, 540, 128, mons)
  assert.strictEqual(mid.x, 960 - 64)
  assert.strictEqual(mid.y, 540 - 64)
})

test("capture: 1.5x logical size and physical coords", () => {
  const mons = jsonFix("monitors-1.5x.json")
  const size = Capture.logicalSize(mons[0])
  assert.strictEqual(size.w, 1920)
  assert.strictEqual(size.h, 1200)
  const phys = Capture.toPhysical(mons[0], 100, 50)
  assert.strictEqual(phys.x, 150)
  assert.strictEqual(phys.y, 75)
})

test("capture: 2x mapping", () => {
  const mons = jsonFix("monitors-2x.json")
  const size = Capture.logicalSize(mons[0])
  assert.strictEqual(size.w, 1280)
  assert.strictEqual(size.h, 800)
  const phys = Capture.toPhysical(mons[0], 10, 10)
  assert.strictEqual(phys.x, 20)
  assert.strictEqual(phys.y, 20)
})

test("capture: straddling window is clipped to the focused monitor", () => {
  const mons = jsonFix("monitors-dual.json")
  const win = jsonFix("activewindow.json")
  const clipped = Capture.windowOnMonitor(win, mons[0])
  assert.strictEqual(clipped.x, 400)
  assert.strictEqual(clipped.y, 200)
  assert.strictEqual(clipped.w, 1920 - 400)
  assert.strictEqual(clipped.h, 800)
})

test("capture: sourceClip stays inside the last 128px frame", () => {
  const frame = { x: 100, y: 100, w: 128, h: 128 }
  const clip = Capture.sourceClip(120, 130, frame, 48)
  assert.ok(clip.x >= 0)
  assert.ok(clip.y >= 0)
  assert.ok(clip.x + clip.w <= 128)
  assert.ok(Capture.frameStillCovers(120, 130, frame, 16))
  assert.strictEqual(Capture.frameStillCovers(10, 10, frame, 16), false)
})

test("capture: cache-bust url uses query counter", () => {
  const url = Capture.cacheBustUrl("/dev/shm/chroma-1/frame0.png", 42, 0)
  assert.ok(url.indexOf("file:///dev/shm/chroma-1/frame0.png") === 0)
  assert.ok(url.indexOf("n=42") >= 0)
})

test("capture: grim hello is pick-mode, protocol hello is live", () => {
  const grim = Capture.capabilitiesFromHello({ backend: "grim", live: false, pickMode: true })
  assert.strictEqual(grim.pickMode, true)
  assert.strictEqual(grim.live, false)
  const ext = Capture.capabilitiesFromHello({ backend: "ext-image-copy-capture-v1", live: true, pickMode: false })
  assert.strictEqual(ext.live, true)
  assert.strictEqual(ext.protocol, true)
})

test("capture: recursion sentinel flags lens chrome in the sample", () => {
  const samples = ["#112233", "#ff2bd6", "#ff2bd6", "#445566"]
  assert.strictEqual(Capture.recursionHit(samples, "#ff2bd6"), true)
  assert.strictEqual(Capture.recursionHit(["#112233", "#445566"], "#ff2bd6"), false)
})

test("capture: ruler is px only", () => {
  assert.strictEqual(Capture.rulerDistance({ x: 0, y: 0 }, { x: 3, y: 4 }), 5)
})

test("capture: json protocol encode/decode", () => {
  const line = Capture.encodeCmd({ cmd: "pick", x: 1, y: 2 }).trim()
  const msg = Capture.parseJsonLine(line)
  assert.strictEqual(msg.cmd, "pick")
  assert.strictEqual(Capture.parseJsonLine("not-json"), null)
})

test("capture: loupe flips near the right edge", () => {
  const pos = Capture.loupePosition(1900, 100, 192, 192, 1920, 1080, 60)
  assert.ok(pos.x + 192 <= 1920)
  assert.strictEqual(Capture.hudAtTop(1000, 1080), true)
  assert.strictEqual(Capture.hudAtTop(100, 1080), false)
})

const summary = passed + " passed, " + failed + " failed"
process.stdout.write(summary + "\n")
process.exit(failed ? 1 : 0)
