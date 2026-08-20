.pragma library

var VERSION = 1
var DEFAULT_LIMIT = 24
var picks = []
var revision = 0

function snapshot() {
    return {
        version: VERSION,
        picks: picks.slice(),
        revision: revision
    }
}

function load(raw) {
    var data = raw
    if (typeof raw === "string") {
        try {
            data = JSON.parse(raw)
        } catch (e) {
            return false
        }
    }
    if (!data || typeof data !== "object")
        return false
    picks = []
    var list = data.picks || []
    for (var i = 0; i < list.length; i++) {
        if (list[i] && list[i].hex)
            picks.push({
                hex: String(list[i].hex).toLowerCase(),
                rgb: list[i].rgb || "",
                hsl: list[i].hsl || "",
                oklch: list[i].oklch || "",
                at: list[i].at || 0
            })
    }
    revision += 1
    return true
}

function serialize() {
    return JSON.stringify({ version: VERSION, picks: picks }, null, 2)
}

function push(entry, limit) {
    var cap = limit || DEFAULT_LIMIT
    if (!entry || !entry.hex)
        return picks.slice()
    var hex = String(entry.hex).toLowerCase()
    var next = []
    for (var i = 0; i < picks.length; i++) {
        if (picks[i].hex !== hex)
            next.push(picks[i])
    }
    next.unshift({
        hex: hex,
        rgb: entry.rgb || "",
        hsl: entry.hsl || "",
        oklch: entry.oklch || "",
        at: entry.at || Date.now()
    })
    if (next.length > cap)
        next = next.slice(0, cap)
    picks = next
    revision += 1
    return picks.slice()
}

function clear() {
    picks = []
    revision += 1
}

function last(n) {
    var count = n || 2
    return picks.slice(0, count)
}

function reset() {
    picks = []
    revision = 0
}
