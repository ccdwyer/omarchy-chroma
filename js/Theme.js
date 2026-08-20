.pragma library
.import "Color.js" as Color

// Palette (k=6) → Omarchy colors.toml token slots.
// Darkest → background, lightest (contrast-fixed) → foreground,
// highest chroma among mid-lightness → accent. Remaining fill ANSI.

var REQUIRED_KEYS = [
    "accent", "cursor", "foreground", "background",
    "selection_foreground", "selection_background",
    "color0", "color1", "color2", "color3", "color4", "color5", "color6", "color7",
    "color8", "color9", "color10", "color11", "color12", "color13", "color14", "color15"
]

function parseHexList(list) {
    var out = []
    if (!list)
        return out
    for (var i = 0; i < list.length; i++) {
        var d = Color.parseColor(list[i].hex || list[i])
        if (d)
            out.push(d)
    }
    return out
}

function uniqueByHex(list) {
    var seen = {}
    var out = []
    for (var i = 0; i < list.length; i++) {
        var h = list[i].hex
        if (seen[h])
            continue
        seen[h] = true
        out.push(list[i])
    }
    return out
}

function sortByLightness(list) {
    var copy = list.slice()
    copy.sort(function (a, b) { return a.oklchL - b.oklchL })
    return copy
}

function pickAccent(colors, background, foreground) {
    var best = null
    var bestScore = -1
    for (var i = 0; i < colors.length; i++) {
        var c = colors[i]
        if (c.hex === background.hex)
            continue
        if (c.hex === foreground.hex)
            continue
        var mid = 1 - Math.abs(c.oklchL - 0.62) * 1.4
        if (mid < 0)
            mid = 0
        var score = c.oklchC * 3 + mid
        if (score > bestScore) {
            bestScore = score
            best = c
        }
    }
    if (best)
        return best
    for (var j = 0; j < colors.length; j++) {
        if (colors[j].hex !== background.hex)
            return colors[j]
    }
    return foreground
}

function deriveRamp(bg, fg) {
    return {
        color0: Color.describe(Color.mixRgb(bg, fg, 0.14)),
        color7: Color.describe(Color.mixRgb(bg, fg, 0.72)),
        color8: Color.describe(Color.mixRgb(bg, fg, 0.28)),
        color15: Color.describe(Color.mixRgb(bg, fg, 0.92))
    }
}

function hueBucket(h) {
    var x = ((h % 360) + 360) % 360
    if (x < 30 || x >= 330)
        return "red"
    if (x < 70)
        return "yellow"
    if (x < 150)
        return "green"
    if (x < 200)
        return "cyan"
    if (x < 260)
        return "blue"
    if (x < 310)
        return "magenta"
    return "red"
}

function fillAnsi(colors, accent, bg, fg) {
    var buckets = { red: null, yellow: null, green: null, cyan: null, blue: null, magenta: null }
    buckets[hueBucket(accent.oklchH)] = accent
    for (var i = 0; i < colors.length; i++) {
        var c = colors[i]
        if (c.hex === bg.hex || c.hex === fg.hex)
            continue
        var b = hueBucket(c.oklchH)
        if (!buckets[b] || c.oklchC > buckets[b].oklchC)
            buckets[b] = c
    }
    function fallback(h, C) {
        var rgb = Color.oklchToRgb(0.65, C || 0.12, h)
        return Color.describe(rgb)
    }
    return {
        red: buckets.red || fallback(25, 0.16),
        yellow: buckets.yellow || fallback(85, 0.13),
        green: buckets.green || fallback(140, 0.13),
        cyan: buckets.cyan || fallback(200, 0.10),
        blue: buckets.blue || accent,
        magenta: buckets.magenta || fallback(320, 0.14)
    }
}

function mapPalette(list) {
    var colors = uniqueByHex(parseHexList(list))
    if (colors.length === 0) {
        colors = [
            Color.describe("#1a1b26"),
            Color.describe("#c0caf5"),
            Color.describe("#7aa2f7"),
            Color.describe("#f7768e"),
            Color.describe("#9ece6a"),
            Color.describe("#e0af68")
        ]
    }
    var byL = sortByLightness(colors)
    var background = byL[0]
    var foreground = byL[byL.length - 1]
    var fgHex = Color.lightenUntilContrast(foreground.hex, background.hex, 4.5)
    foreground = Color.describe(fgHex)
    var accent = pickAccent(colors, background, foreground)
    if (Color.contrastRatio(accent.hex, background.hex) < 2.2) {
        var lifted = Color.oklchToRgb(Math.max(accent.oklchL, 0.62), Math.max(accent.oklchC, 0.1), accent.oklchH)
        accent = Color.describe(lifted)
    }
    var ramp = deriveRamp(background, foreground)
    var ansi = fillAnsi(colors, accent, background, foreground)
    var selectionBg = accent
    var selectionFg = Color.describe(Color.lightenUntilContrast(background.hex, accent.hex, 4.5))
    var cursor = Color.describe(Color.mixRgb(foreground, accent, 0.18))

    function bright(c) {
        var rgb = Color.oklchToRgb(Math.min(0.86, c.oklchL + 0.08), c.oklchC * 0.95, c.oklchH)
        return Color.describe(rgb)
    }

    var tokens = {
        accent: accent.hex,
        cursor: cursor.hex,
        foreground: foreground.hex,
        background: background.hex,
        selection_foreground: selectionFg.hex,
        selection_background: selectionBg.hex,
        color0: ramp.color0.hex,
        color1: ansi.red.hex,
        color2: ansi.green.hex,
        color3: ansi.yellow.hex,
        color4: ansi.blue.hex,
        color5: ansi.magenta.hex,
        color6: ansi.cyan.hex,
        color7: ramp.color7.hex,
        color8: ramp.color8.hex,
        color9: bright(ansi.red).hex,
        color10: bright(ansi.green).hex,
        color11: bright(ansi.yellow).hex,
        color12: bright(ansi.blue).hex,
        color13: bright(ansi.magenta).hex,
        color14: bright(ansi.cyan).hex,
        color15: ramp.color15.hex
    }
    return {
        tokens: tokens,
        roles: {
            background: background.hex,
            foreground: foreground.hex,
            accent: accent.hex
        },
        contrast: Color.contrastInfo(foreground.hex, background.hex)
    }
}

function colorsToml(tokens) {
    var lines = []
    function line(k) {
        lines.push(k + " = \"" + tokens[k] + "\"")
    }
    line("accent")
    line("cursor")
    line("foreground")
    line("background")
    lines.push("selection_foreground = \"" + tokens.selection_foreground + "\"")
    lines.push("selection_background = \"" + tokens.selection_background + "\"")
    lines.push("")
    for (var i = 0; i < 16; i++)
        line("color" + i)
    return lines.join("\n") + "\n"
}

function hyprlandConf(tokens) {
    var strip = String(tokens.accent).replace("#", "")
    return "$activeBorderColor = rgb(" + strip + ")\n\n" +
        "general {\n    col.active_border = $activeBorderColor\n}\n\n" +
        "group {\n    col.border_active = $activeBorderColor\n}\n"
}

function alacrittyToml(tokens) {
    function q(k) { return "\"" + tokens[k] + "\"" }
    return "[colors.primary]\n" +
        "background = " + q("background") + "\n" +
        "foreground = " + q("foreground") + "\n\n" +
        "[colors.cursor]\n" +
        "text = " + q("background") + "\n" +
        "cursor = " + q("cursor") + "\n\n" +
        "[colors.selection]\n" +
        "text = " + q("selection_foreground") + "\n" +
        "background = " + q("selection_background") + "\n\n" +
        "[colors.normal]\n" +
        "black = " + q("color0") + "\n" +
        "red = " + q("color1") + "\n" +
        "green = " + q("color2") + "\n" +
        "yellow = " + q("color3") + "\n" +
        "blue = " + q("color4") + "\n" +
        "magenta = " + q("color5") + "\n" +
        "cyan = " + q("color6") + "\n" +
        "white = " + q("color7") + "\n\n" +
        "[colors.bright]\n" +
        "black = " + q("color8") + "\n" +
        "red = " + q("color9") + "\n" +
        "green = " + q("color10") + "\n" +
        "yellow = " + q("color11") + "\n" +
        "blue = " + q("color12") + "\n" +
        "magenta = " + q("color13") + "\n" +
        "cyan = " + q("color14") + "\n" +
        "white = " + q("color15") + "\n"
}

function generate(list) {
    var mapped = mapPalette(list)
    var tokens = mapped.tokens
    return {
        name: "chroma-preview",
        tokens: tokens,
        roles: mapped.roles,
        contrast: mapped.contrast,
        files: {
            "colors.toml": colorsToml(tokens),
            "hyprland.conf": hyprlandConf(tokens),
            "alacritty.toml": alacrittyToml(tokens)
        }
    }
}

function validateTokens(tokens) {
    if (!tokens || typeof tokens !== "object")
        return { ok: false, error: "missing tokens" }
    for (var i = 0; i < REQUIRED_KEYS.length; i++) {
        var k = REQUIRED_KEYS[i]
        if (!Color.normalizeHex(tokens[k]))
            return { ok: false, error: "bad or missing " + k }
    }
    var info = Color.contrastInfo(tokens.foreground, tokens.background)
    if (info.ratio < 3)
        return { ok: false, error: "foreground/background contrast " + info.ratioText }
    return { ok: true, contrast: info }
}

function parseColorsToml(text) {
    var tokens = {}
    var lines = String(text || "").split("\n")
    for (var i = 0; i < lines.length; i++) {
        var m = lines[i].match(/^\s*([A-Za-z0-9_]+)\s*=\s*"?(#[0-9A-Fa-f]{6})"?/)
        if (m)
            tokens[m[1]] = m[2].toLowerCase()
    }
    return tokens
}
