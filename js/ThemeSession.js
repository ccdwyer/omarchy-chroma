.pragma library

// Transactional preview revert. Snapshot the active theme at the start of
// every preview while not live; clear after a successful revert so the next
// preview cannot restore a stale name.

var original = ""
var live = false

function reset() {
    original = ""
    live = false
}

function beginPreview(currentName, alreadyLive) {
    if (alreadyLive)
        return original
    var name = String(currentName || "").trim()
    if (name && name !== "chroma-preview")
        original = name
    return original
}

function markApplied() {
    live = true
}

function markApplyFailed() {
    live = false
}

function afterSuccessfulRevert() {
    original = ""
    live = false
}

function snapshot() {
    return { original: original, live: live }
}

function serialize() {
    return JSON.stringify({ original: original, live: live })
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
    if (!data || typeof data !== "object") {
        original = ""
        live = false
        return false
    }
    original = data.original ? String(data.original) : ""
    if (original === "chroma-preview")
        original = ""
    live = !!data.live
    return true
}
