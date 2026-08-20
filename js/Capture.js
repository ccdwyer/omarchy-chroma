.pragma library

// Frame transport + coordinate helpers. Chromad owns capture;
// QML crops the 128px region so cursor micro-movement needs no recapture.

var REGION = 128
var STREAM_HZ = 30
var CURSOR_HZ = 60
var LOUPE_OFFSET = 60
var SENTINEL = "#ff2bd6"

function parseJsonLine(line) {
    if (!line)
        return null
    var s = String(line).trim()
    if (!s.length)
        return null
    try {
        return JSON.parse(s)
    } catch (e) {
        return null
    }
}

function encodeCmd(obj) {
    return JSON.stringify(obj) + "\n"
}

function clamp(n, lo, hi) {
    if (n < lo)
        return lo
    if (n > hi)
        return hi
    return n
}

function parseCursorText(text) {
    var s = String(text || "").trim()
    if (!s.length)
        return null
    try {
        var j = JSON.parse(s)
        if (typeof j.x === "number" && typeof j.y === "number")
            return { x: j.x, y: j.y }
    } catch (e) {}
    var m = s.match(/(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)/)
    if (m)
        return { x: Number(m[1]), y: Number(m[2]) }
    return null
}

function monitorContaining(monitors, x, y) {
    var list = monitors || []
    var found = null
    for (var i = 0; i < list.length; i++) {
        var m = list[i]
        var mx = Number(m.x) || 0
        var my = Number(m.y) || 0
        var scale = Number(m.scale) || 1
        if (scale <= 0)
            scale = 1
        var lw = (Number(m.width) || 0) / scale
        var lh = (Number(m.height) || 0) / scale
        if (x >= mx && y >= my && x < mx + lw && y < my + lh)
            return m
        if (m.focused)
            found = m
    }
    return found || list[0] || null
}

function logicalSize(monitor) {
    if (!monitor)
        return { w: 0, h: 0, scale: 1 }
    var scale = Number(monitor.scale) || 1
    if (scale <= 0)
        scale = 1
    return {
        w: (Number(monitor.width) || 0) / scale,
        h: (Number(monitor.height) || 0) / scale,
        scale: scale
    }
}

function toPhysical(monitor, lx, ly) {
    var scale = monitor && Number(monitor.scale) ? Number(monitor.scale) : 1
    var mx = monitor ? Number(monitor.x) || 0 : 0
    var my = monitor ? Number(monitor.y) || 0 : 0
    return {
        x: Math.round((lx - mx) * scale),
        y: Math.round((ly - my) * scale),
        scale: scale,
        name: monitor && monitor.name ? monitor.name : ""
    }
}

function regionAround(x, y, size, monitors) {
    var s = size || REGION
    var half = Math.floor(s / 2)
    var mon = monitorContaining(monitors, x, y)
    var dim = logicalSize(mon)
    var mx = mon ? Number(mon.x) || 0 : 0
    var my = mon ? Number(mon.y) || 0 : 0
    var rx = Math.round(x - half)
    var ry = Math.round(y - half)
    if (dim.w > 0) {
        rx = clamp(rx, mx, mx + dim.w - s)
        ry = clamp(ry, my, my + dim.h - s)
    }
    return {
        x: rx,
        y: ry,
        w: s,
        h: s,
        monitor: mon ? (mon.name || "") : "",
        scale: dim.scale
    }
}

function intersectRects(a, b) {
    var x = Math.max(a.x, b.x)
    var y = Math.max(a.y, b.y)
    var r = Math.min(a.x + a.w, b.x + b.w)
    var btm = Math.min(a.y + a.h, b.y + b.h)
    if (r <= x || btm <= y)
        return { x: x, y: y, w: 0, h: 0 }
    return { x: x, y: y, w: r - x, h: btm - y }
}

function windowOnMonitor(win, monitor) {
    if (!win)
        return null
    var at = win.at || [win.x || 0, win.y || 0]
    var size = win.size || [win.w || win.width || 0, win.h || win.height || 0]
    var rect = { x: Number(at[0]) || 0, y: Number(at[1]) || 0, w: Number(size[0]) || 0, h: Number(size[1]) || 0 }
    if (!monitor)
        return rect
    var dim = logicalSize(monitor)
    var bounds = {
        x: Number(monitor.x) || 0,
        y: Number(monitor.y) || 0,
        w: dim.w,
        h: dim.h
    }
    return intersectRects(rect, bounds)
}

function loupePosition(cursorX, cursorY, loupeW, loupeH, screenW, screenH, offset) {
    var off = offset === undefined ? LOUPE_OFFSET : offset
    var x = cursorX + off
    var y = cursorY - loupeH - off / 2
    if (x + loupeW > screenW - 8)
        x = cursorX - loupeW - off
    if (x < 8)
        x = 8
    if (y < 8)
        y = cursorY + off
    if (y + loupeH > screenH - 8)
        y = screenH - loupeH - 8
    return { x: x, y: y }
}

function hudAtTop(cursorY, screenH) {
    return cursorY > screenH * 0.72
}

function frameScale(frame) {
    var s = frame && frame.scale !== undefined ? Number(frame.scale) : 1
    if (!s || s <= 0)
        return 1
    return s
}

function sourceClip(cursorX, cursorY, frame, view) {
    if (!frame)
        return { x: 0, y: 0, w: view || 48, h: view || 48 }
    var scale = frameScale(frame)
    var v = (view || 48) * scale
    var fx = Number(frame.x) || 0
    var fy = Number(frame.y) || 0
    var fw = (Number(frame.w) || REGION) * scale
    var fh = (Number(frame.h) || REGION) * scale
    var x = (cursorX - fx) * scale - v / 2
    var y = (cursorY - fy) * scale - v / 2
    x = clamp(x, 0, Math.max(0, fw - v))
    y = clamp(y, 0, Math.max(0, fh - v))
    return { x: x, y: y, w: Math.min(v, fw), h: Math.min(v, fh) }
}

function frameStillCovers(cursorX, cursorY, frame, pad) {
    if (!frame)
        return false
    var p = pad === undefined ? 16 : pad
    return cursorX >= frame.x + p &&
        cursorY >= frame.y + p &&
        cursorX <= frame.x + frame.w - p &&
        cursorY <= frame.y + frame.h - p
}

function cacheBustUrl(path, gen, slot) {
    var p = String(path || "")
    if (p.indexOf("file://") !== 0)
        p = "file://" + p
    var n = Number(gen) || 0
    var s = slot === undefined ? 0 : slot
    if (p.indexOf("?") >= 0)
        return p + "&n=" + n + "&s=" + s
    return p + "?n=" + n + "&s=" + s
}

function hexClose(a, b, tol) {
    function ch(h, i) { return parseInt(String(h).slice(i, i + 2), 16) }
    var ha = String(a || "").replace("#", "")
    var hb = String(b || "").replace("#", "")
    if (ha.length < 6 || hb.length < 6)
        return false
    var t = tol === undefined ? 12 : tol
    return Math.abs(ch(ha, 0) - ch(hb, 0)) <= t &&
        Math.abs(ch(ha, 2) - ch(hb, 2)) <= t &&
        Math.abs(ch(ha, 4) - ch(hb, 4)) <= t
}

function recursionHit(samples, sentinel) {
    var mark = sentinel || SENTINEL
    if (!samples || !samples.length)
        return false
    var hits = 0
    for (var i = 0; i < samples.length; i++) {
        if (hexClose(samples[i], mark, 10))
            hits += 1
    }
    return hits > samples.length * 0.08
}

function capabilitiesFromHello(msg) {
    var m = msg || {}
    var backend = m.backend || "none"
    var live = m.live === true
    var pickMode = m.pickMode === true || !live
    if (backend === "grim")
        pickMode = true
    return {
        backend: backend,
        live: live && !pickMode,
        pickMode: pickMode,
        ocr: m.ocr === true,
        qr: m.qr === true,
        protocol: backend === "ext-image-copy-capture-v1" || backend === "wlr-screencopy"
    }
}

function rulerDistance(a, b) {
    if (!a || !b)
        return 0
    var dx = b.x - a.x
    var dy = b.y - a.y
    return Math.round(Math.sqrt(dx * dx + dy * dy))
}

function zoomClip(cursorX, cursorY, frame, factor) {
    var z = factor || 8
    if (z < 1)
        z = 1
    var vw = Math.max(8, Math.round((frame && frame.w ? frame.w : 128) / z))
    var vh = Math.max(8, Math.round((frame && frame.h ? frame.h : 128) / z))
    return sourceClip(cursorX, cursorY, frame, vw)
}
