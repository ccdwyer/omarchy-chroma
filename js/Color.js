.pragma library

// Color science used by the lens HUD. No helper round-trip.
// sRGB in, HEX / RGB / HSL / OKLCH / WCAG contrast out.

function clamp(n, lo, hi) {
    if (n < lo)
        return lo
    if (n > hi)
        return hi
    return n
}

function round(n, digits) {
    var p = Math.pow(10, digits || 0)
    return Math.round(n * p) / p
}

function hexByte(n) {
    var v = clamp(Math.round(n), 0, 255)
    var s = v.toString(16)
    if (s.length < 2)
        s = "0" + s
    return s
}

function normalizeHex(input) {
    if (input === undefined || input === null)
        return null
    var s = String(input).trim()
    if (s.charAt(0) === "#")
        s = s.slice(1)
    if (s.length === 3) {
        s = s.charAt(0) + s.charAt(0) + s.charAt(1) + s.charAt(1) + s.charAt(2) + s.charAt(2)
    } else if (s.length === 8) {
        s = s.slice(0, 6)
    }
    if (!/^[0-9a-fA-F]{6}$/.test(s))
        return null
    return "#" + s.toLowerCase()
}

function hexToRgb(hex) {
    var h = normalizeHex(hex)
    if (!h)
        return null
    return {
        r: parseInt(h.slice(1, 3), 16),
        g: parseInt(h.slice(3, 5), 16),
        b: parseInt(h.slice(5, 7), 16)
    }
}

function rgbToHex(r, g, b) {
    return "#" + hexByte(r) + hexByte(g) + hexByte(b)
}

function rgbToHsl(r, g, b) {
    var R = r / 255
    var G = g / 255
    var B = b / 255
    var max = Math.max(R, G, B)
    var min = Math.min(R, G, B)
    var l = (max + min) / 2
    var d = max - min
    var h = 0
    var s = 0
    if (d !== 0) {
        s = l > 0.5 ? d / (2 - max - min) : d / (max + min)
        if (max === R)
            h = ((G - B) / d + (G < B ? 6 : 0)) / 6
        else if (max === G)
            h = ((B - R) / d + 2) / 6
        else
            h = ((R - G) / d + 4) / 6
    }
    return { h: h * 360, s: s * 100, l: l * 100 }
}

function hslToRgb(h, s, l) {
    var H = ((h % 360) + 360) % 360 / 360
    var S = clamp(s / 100, 0, 1)
    var L = clamp(l / 100, 0, 1)
    if (S === 0) {
        var g = Math.round(L * 255)
        return { r: g, g: g, b: g }
    }
    function hue2rgb(p, q, t) {
        if (t < 0)
            t += 1
        if (t > 1)
            t -= 1
        if (t < 1 / 6)
            return p + (q - p) * 6 * t
        if (t < 1 / 2)
            return q
        if (t < 2 / 3)
            return p + (q - p) * (2 / 3 - t) * 6
        return p
    }
    var q = L < 0.5 ? L * (1 + S) : L + S - L * S
    var p = 2 * L - q
    return {
        r: Math.round(hue2rgb(p, q, H + 1 / 3) * 255),
        g: Math.round(hue2rgb(p, q, H) * 255),
        b: Math.round(hue2rgb(p, q, H - 1 / 3) * 255)
    }
}

function srgbToLinear(c) {
    var x = c / 255
    if (x <= 0.04045)
        return x / 12.92
    return Math.pow((x + 0.055) / 1.055, 2.4)
}

function linearToSrgb(x) {
    var y = x <= 0.0031308 ? 12.92 * x : 1.055 * Math.pow(Math.max(0, x), 1 / 2.4) - 0.055
    return clamp(Math.round(y * 255), 0, 255)
}

function rgbToOklab(r, g, b) {
    var lr = srgbToLinear(r)
    var lg = srgbToLinear(g)
    var lb = srgbToLinear(b)
    var l = 0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb
    var m = 0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb
    var s = 0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb
    var l_ = Math.cbrt(l)
    var m_ = Math.cbrt(m)
    var s_ = Math.cbrt(s)
    return {
        L: 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_,
        a: 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_,
        b: 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_
    }
}

function oklabToRgb(L, a, b) {
    var l_ = L + 0.3963377774 * a + 0.2158037573 * b
    var m_ = L - 0.1055613458 * a - 0.0638541728 * b
    var s_ = L - 0.0894841775 * a - 1.2914855480 * b
    var l = l_ * l_ * l_
    var m = m_ * m_ * m_
    var s = s_ * s_ * s_
    var lr = +4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s
    var lg = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s
    var lb = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s
    return {
        r: linearToSrgb(lr),
        g: linearToSrgb(lg),
        b: linearToSrgb(lb)
    }
}

function oklabToOklch(lab) {
    var C = Math.sqrt(lab.a * lab.a + lab.b * lab.b)
    var h = Math.atan2(lab.b, lab.a) * 180 / Math.PI
    if (h < 0)
        h += 360
    return { L: lab.L, C: C, h: h }
}

function oklchToOklab(L, C, h) {
    var rad = (h || 0) * Math.PI / 180
    return {
        L: L,
        a: C * Math.cos(rad),
        b: C * Math.sin(rad)
    }
}

function rgbToOklch(r, g, b) {
    return oklabToOklch(rgbToOklab(r, g, b))
}

function oklchToRgb(L, C, h) {
    var lab = oklchToOklab(L, C, h)
    return oklabToRgb(lab.L, lab.a, lab.b)
}

function relativeLuminance(r, g, b) {
    var R = srgbToLinear(r)
    var G = srgbToLinear(g)
    var B = srgbToLinear(b)
    return 0.2126 * R + 0.7152 * G + 0.0722 * B
}

function contrastRatio(a, b) {
    var c1 = typeof a === "string" ? hexToRgb(a) : a
    var c2 = typeof b === "string" ? hexToRgb(b) : b
    if (!c1 || !c2)
        return 0
    var l1 = relativeLuminance(c1.r, c1.g, c1.b)
    var l2 = relativeLuminance(c2.r, c2.g, c2.b)
    var hi = Math.max(l1, l2)
    var lo = Math.min(l1, l2)
    return (hi + 0.05) / (lo + 0.05)
}

function wcagRating(ratio, large) {
    if (large) {
        if (ratio >= 4.5)
            return "AAA"
        if (ratio >= 3)
            return "AA"
        return "fail"
    }
    if (ratio >= 7)
        return "AAA"
    if (ratio >= 4.5)
        return "AA"
    return "fail"
}

function contrastInfo(a, b) {
    var ratio = contrastRatio(a, b)
    return {
        ratio: ratio,
        ratioText: round(ratio, 2).toFixed(2) + ":1",
        aa: ratio >= 4.5,
        aaLarge: ratio >= 3,
        aaa: ratio >= 7,
        aaaLarge: ratio >= 4.5,
        rating: wcagRating(ratio, false),
        ratingLarge: wcagRating(ratio, true)
    }
}

function mixRgb(a, b, t) {
    return {
        r: Math.round(a.r + (b.r - a.r) * t),
        g: Math.round(a.g + (b.g - a.g) * t),
        b: Math.round(a.b + (b.b - a.b) * t)
    }
}

function lightenUntilContrast(fg, bg, target) {
    var f = typeof fg === "string" ? hexToRgb(fg) : { r: fg.r, g: fg.g, b: fg.b }
    var b = typeof bg === "string" ? hexToRgb(bg) : bg
    var white = { r: 255, g: 255, b: 255 }
    var black = { r: 0, g: 0, b: 0 }
    if (contrastRatio(f, b) >= target)
        return rgbToHex(f.r, f.g, f.b)
    var toward = relativeLuminance(b.r, b.g, b.b) < 0.5 ? white : black
    for (var i = 1; i <= 20; i++) {
        var mixed = mixRgb(f, toward, i / 20)
        if (contrastRatio(mixed, b) >= target)
            return rgbToHex(mixed.r, mixed.g, mixed.b)
    }
    return rgbToHex(toward.r, toward.g, toward.b)
}

function chromaOf(hex) {
    var rgb = hexToRgb(hex)
    if (!rgb)
        return 0
    return rgbToOklch(rgb.r, rgb.g, rgb.b).C
}

function lightnessOf(hex) {
    var rgb = hexToRgb(hex)
    if (!rgb)
        return 0
    return rgbToOklch(rgb.r, rgb.g, rgb.b).L
}

function formatRgb(r, g, b) {
    return "rgb(" + Math.round(r) + ", " + Math.round(g) + ", " + Math.round(b) + ")"
}

function formatHsl(h, s, l) {
    return "hsl(" + round(h, 1) + ", " + round(s, 1) + "%, " + round(l, 1) + "%)"
}

function formatOklch(L, C, h) {
    return "oklch(" + round(L, 3) + " " + round(C, 3) + " " + round(h, 1) + ")"
}

function describe(input) {
    var rgb = typeof input === "string" ? hexToRgb(input) : input
    if (!rgb)
        return null
    var hex = rgbToHex(rgb.r, rgb.g, rgb.b)
    var hsl = rgbToHsl(rgb.r, rgb.g, rgb.b)
    var oklch = rgbToOklch(rgb.r, rgb.g, rgb.b)
    var lum = relativeLuminance(rgb.r, rgb.g, rgb.b)
    return {
        hex: hex,
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
        rgb: formatRgb(rgb.r, rgb.g, rgb.b),
        h: hsl.h,
        s: hsl.s,
        l: hsl.l,
        hsl: formatHsl(hsl.h, hsl.s, hsl.l),
        oklchL: oklch.L,
        oklchC: oklch.C,
        oklchH: oklch.h,
        oklch: formatOklch(oklch.L, oklch.C, oklch.h),
        luminance: lum
    }
}

function parseColor(input) {
    if (!input)
        return null
    if (typeof input === "object" && input.r !== undefined)
        return describe(input)
    var s = String(input).trim()
    var hex = normalizeHex(s)
    if (hex)
        return describe(hex)
    var m = s.match(/^rgb\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*\)$/i)
    if (m)
        return describe({ r: Number(m[1]), g: Number(m[2]), b: Number(m[3]) })
    return null
}

function swatchFromRgba(r, g, b, a) {
    var d = describe({ r: r, g: g, b: b })
    if (d)
        d.a = a === undefined ? 255 : a
    return d
}
