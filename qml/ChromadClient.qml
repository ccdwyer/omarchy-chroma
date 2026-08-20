import QtQuick
import Quickshell
import Quickshell.Io
import "../js/Capture.js" as Capture

Item {
  id: root

  property string pluginDir: ""
  property bool ready: false
  property bool pickMode: true
  property bool live: false
  property bool ocrAvailable: false
  property bool qrAvailable: false
  property bool grimAvailable: false
  property string backend: "none"
  property string oneshotBackend: "none"
  property string shmDir: ""
  property string frameUrl: ""
  property int frameGen: 0
  property var frameRect: ({ x: 0, y: 0, w: 128, h: 128 })
  property var pixel: null
  property var palette: []
  property string lastError: ""
  property string lastOcr: ""
  property string lastQr: ""
  property bool running: proc.running
  property bool usingCompat: false

  signal hello(var msg)
  signal frame(var msg)
  signal picked(var msg)
  signal paletted(var msg)
  signal ocrText(string text)
  signal qrText(string text)
  signal failed(string error)

  readonly property string binPath: pluginDir + "/bin/chromad"
  readonly property string compatPath: pluginDir + "/compat/chromad.sh"

  function start() {
    if (proc.running)
      return
    whichProc.running = true
  }

  function stop() {
    root.send({ cmd: "stop_stream" })
    root.send({ cmd: "quit" })
    Qt.callLater(function() {
      if (proc.running)
        proc.running = false
    })
  }

  function send(obj) {
    if (!proc.running || !proc.stdinEnabled)
      return
    proc.write(Capture.encodeCmd(obj))
  }

  function sendCursor(x, y) {
    root.send({ cmd: "cursor", x: x, y: y })
  }

  function startStream() { root.send({ cmd: "start_stream" }) }
  function stopStream() { root.send({ cmd: "stop_stream" }) }
  function pick() { root.send({ cmd: "pick" }) }
  function oneshot(kind) { root.send({ cmd: "oneshot", kind: kind || "monitor" }) }
  function requestPalette(source) { root.send({ cmd: "palette", source: source || "window" }) }
  function requestOcr() { root.send({ cmd: "ocr", source: "monitor" }) }
  function requestQr() { root.send({ cmd: "qr", source: "monitor" }) }
  function freeze(factor) { root.send({ cmd: "freeze", factor: factor || 8 }) }
  function unfreeze() { root.send({ cmd: "unfreeze" }) }
  function writeTheme(dir, files) { root.send({ cmd: "write_theme", dir: dir, files: files }) }
  function validateTheme(dir) { root.send({ cmd: "validate_theme", dir: dir }) }

  function handleLine(line) {
    var msg = Capture.parseJsonLine(line)
    if (!msg)
      return
    if (msg.ok === false) {
      root.lastError = msg.error || "chromad error"
      root.failed(root.lastError)
      return
    }
    var ev = msg.event || ""
    if (ev === "hello") {
      var cap = Capture.capabilitiesFromHello(msg)
      root.backend = cap.backend
      root.oneshotBackend = msg.oneshotBackend || cap.backend
      root.live = cap.live
      root.pickMode = cap.pickMode
      root.ocrAvailable = cap.ocr
      root.qrAvailable = cap.qr
      root.grimAvailable = msg.grim === true
      root.shmDir = msg.shm || ""
      root.ready = true
      root.hello(msg)
    } else if (ev === "frame") {
      root.frameRect = { x: msg.x || 0, y: msg.y || 0, w: msg.w || 128, h: msg.h || 128 }
      root.frameGen = msg.n || (root.frameGen + 1)
      root.frameUrl = Capture.cacheBustUrl(msg.path, root.frameGen, msg.slot || 0)
      if (msg.pixel)
        root.pixel = msg.pixel
      root.frame(msg)
    } else if (ev === "pick") {
      root.pixel = msg.pixel || msg
      root.picked(msg)
    } else if (ev === "palette") {
      root.palette = msg.colors || []
      root.paletted(msg)
    } else if (ev === "ocr") {
      root.lastOcr = msg.text || ""
      root.ocrText(root.lastOcr)
    } else if (ev === "qr") {
      root.lastQr = msg.text || ""
      root.qrText(root.lastQr)
    } else if (ev === "stream") {
      if (msg.pickMode === true)
        root.pickMode = true
    }
  }

  Process {
    id: whichProc
    running: false
    command: ["sh", "-c", "if test -x \"$1\"; then echo binary; elif test -x \"$2\"; then echo compat; else echo missing; fi", "sh", root.binPath, root.compatPath]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var out = String(text || "").trim()
        if (out === "binary") {
          root.usingCompat = false
          proc.command = [root.binPath, "--serve"]
        } else {
          root.usingCompat = true
          proc.command = ["sh", root.compatPath, "--serve"]
        }
        proc.stdinEnabled = true
        proc.running = true
      }
    }
  }

  Process {
    id: proc
    running: false
    stdinEnabled: true
    stdout: SplitParser {
      onRead: function(data) { root.handleLine(data) }
    }
    stderr: SplitParser {
      onRead: function(data) {
        if (data && String(data).length)
          root.lastError = String(data)
      }
    }
    onExited: function() {
      root.ready = false
      root.live = false
    }
  }
}
