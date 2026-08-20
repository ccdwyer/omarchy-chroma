import QtQuick
import Quickshell
import Quickshell.Io
import Quickshell.Wayland
import qs.Commons
import qs.Ui
import "js/Color.js" as ColorMath
import "js/Theme.js" as Theme
import "js/ThemeSession.js" as ThemeSession
import "js/History.js" as History
import "js/Capture.js" as Capture
import "qml"

Item {
  id: root
  property string moduleName: "io.github.chris.chroma"

  property var shell: null
  property var manifest: null
  property var pluginRegistry: null
  property string omarchyPath: Quickshell.env("OMARCHY_PATH") || ""
  property string pluginId: "io.github.chris.chroma"
  property bool opened: false
  property bool capturing: false
  property bool helpOpen: false
  property bool paletteOpen: false
  property bool rulerOpen: false
  property bool freezeOpen: false
  property bool historyOpen: false
  property bool themeLive: false
  property bool awaitingThemeValid: false
  property string themeApplyName: ""
  property string themeSetErr: ""
  property bool paletteBusy: false
  property int loupeOffset: 60
  property int historyLimit: 24
  property int zoomDefault: 4
  property int zoom: 4
  property real cursorX: 0
  property real cursorY: 0
  property var pixel: null
  property var paletteColors: []
  property string originalTheme: ""
  property string pendingAction: ""
  property string ocrText: ""
  property string qrText: ""
  property string toast: ""
  property string hint: ""
  property int historyRev: 0
  property int hideAckTries: 0
  property int motionMs: reduceMotion ? 0 : 150

  property color background: Color.menu.background
  property color foreground: Color.menu.text
  property color border: Color.menu.border
  property color scrim: Color.menu.scrim
  property color accent: Color.accent
  property var borderSpec: Border.surfaceSpec("menu", "border", border, Math.max(1, Style.space(2)))
  property string fontFamily: Style.font.menuFamily

  readonly property string pluginDir: {
    var u = String(Qt.resolvedUrl("."))
    if (u.indexOf("file://") === 0)
      u = u.slice(7)
    if (u.length > 1 && u.charAt(u.length - 1) === "/")
      u = u.slice(0, u.length - 1)
    return u
  }
  readonly property string home: Quickshell.env("HOME") || "/tmp"
  readonly property string stateDir: {
    var xdg = Quickshell.env("XDG_STATE_HOME")
    if (xdg && xdg.length)
      return xdg + "/chroma"
    return home + "/.local/state/chroma"
  }
  readonly property string historyPath: stateDir + "/history.json"
  readonly property string sessionPath: stateDir + "/theme-session.json"
  readonly property string themeDir: home + "/.config/omarchy/themes/chroma-preview"
  readonly property string themeNamePath: home + "/.config/omarchy/current/theme.name"
  readonly property bool reduceMotion: {
    try {
      if (Style && Style.reduceMotion)
        return true
    } catch (e) {}
    try {
      if (Quickshell.env("OMARCHY_REDUCED_MOTION") === "1")
        return true
    } catch (e2) {}
    return false
  }
  readonly property var historySnap: {
    var _r = root.historyRev
    return History.snapshot()
  }
  readonly property string hexA: historySnap.picks.length > 0 ? historySnap.picks[0].hex : ""
  readonly property string hexB: historySnap.picks.length > 1 ? historySnap.picks[1].hex : ""
  readonly property var picks: historySnap.picks

  function open(payloadJson) {
    root.applyPayload(payloadJson)
    root.opened = true
    root.helpOpen = false
    root.rulerOpen = false
    root.freezeOpen = false
    root.capturing = false
    root.hint = ""
    historyFile.reload()
    sessionFile.reload()
    themeNameFile.reload()
    client.start()
    Qt.callLater(function() {
      client.startStream()
      keyCatcher.forceActiveFocus()
    })
  }

  function close() {
    client.stopStream()
    if (root.freezeOpen)
      client.unfreeze()
    root.pendingAction = ""
    root.opened = false
    root.helpOpen = false
    root.paletteOpen = false
    root.rulerOpen = false
    root.freezeOpen = false
    root.capturing = false
    ruler.reset()
  }

  function toggle(payloadJson) {
    if (root.opened)
      root.close()
    else
      root.open(payloadJson || "{}")
  }

  function pick(arg) {
    root.withHiddenCapture("pick")
    return "ok"
  }

  function palette(arg) {
    var action = "palette"
    try {
      if (arg && String(arg).length && String(arg) !== "{}") {
        var p = JSON.parse(arg)
        if (p && p.source === "monitor")
          action = "palette-monitor"
      }
    } catch (e) {}
    root.withHiddenCapture(action)
    return "ok"
  }

  function revert(arg) {
    root.revertTheme()
    return "ok"
  }

  function status(arg) {
    return root.statusJson()
  }

  function applyPayload(payloadJson) {
    try {
      var payload = payloadJson && String(payloadJson).length ? JSON.parse(payloadJson) : {}
      if (payload.loupeOffset)
        root.loupeOffset = Number(payload.loupeOffset)
      if (payload.historyLimit)
        root.historyLimit = Number(payload.historyLimit)
      if (payload.zoomDefault)
        root.zoomDefault = Number(payload.zoomDefault)
      if (payload.mode === "history")
        root.historyOpen = true
      if (payload.mode === "palette")
        root.paletteOpen = true
    } catch (e) {}
  }

  function withHiddenCapture(action) {
    root.pendingAction = action
    var alreadyHidden = !panel.visible
    root.capturing = true
    if (alreadyHidden)
      root.ackOverlayHidden()
  }

  function ackOverlayHidden() {
    if (!root.pendingAction.length)
      return
    root.hideAckTries = 0
    hideAckProc.running = false
    hideAckProc.running = true
  }

  function chromaStillMapped(text) {
    var t = String(text || "")
    return t.indexOf('"namespace":"chroma"') !== -1 || t.indexOf('"namespace": "chroma"') !== -1
  }

  function runPending() {
    var a = root.pendingAction
    root.pendingAction = ""
    if (a === "pick")
      client.pick()
    else if (a === "palette") {
      root.paletteBusy = true
      client.requestPalette("window")
    } else if (a === "palette-monitor") {
      root.paletteBusy = true
      client.requestPalette("monitor")
    } else if (a === "ocr")
      client.requestOcr()
    else if (a === "qr")
      client.requestQr()
    else if (a === "freeze")
      client.freeze(root.zoom)
    else if (a === "oneshot")
      client.oneshot("region")
  }

  function showAgain() {
    root.capturing = false
    Qt.callLater(function() { keyCatcher.forceActiveFocus() })
  }

  function onPicked(msg) {
    root.showAgain()
    var d = ColorMath.parseColor((msg && msg.hex) || (msg && msg.pixel && msg.pixel.hex) || "")
    if (!d && msg && msg.pixel)
      d = ColorMath.parseColor(msg.pixel)
    if (!d)
      return
    root.pixel = d
    History.push(d, root.historyLimit)
    root.historyRev = History.revision
    historyFile.setText(History.serialize())
    root.copyHex(false)
  }

  function copyHex(toastAlways) {
    if (!root.pixel || !root.pixel.hex)
      return
    copyProc.command = ["sh", "-c", "printf %s \"$1\" | wl-copy", "chroma-copy", root.pixel.hex]
    copyProc.running = true
    if (toastAlways !== false)
      root.toast = root.pixel.hex + " copied"
    else
      root.toast = root.pixel.hex + " copied"
  }

  function makeTheme() {
    if (!root.paletteColors || root.paletteColors.length === 0)
      return
    var gen = Theme.generate(root.paletteColors)
    var check = Theme.validateTokens(gen.tokens)
    if (!check.ok) {
      root.toast = "theme invalid: " + check.error
      return
    }
    if (!root.themeLive)
      root.snapshotTheme()
    if (!ThemeSession.hasRevertTarget()) {
      root.toast = "no revert target — refusing preview"
      return
    }
    root.originalTheme = ThemeSession.snapshot().original
    writeThemeProc.running = false
    writeThemeProc.command = [
      "sh", "-c",
      "mkdir -p \"$1\" && printf '%s' \"$2\" > \"$1/colors.toml\" && printf '%s' \"$3\" > \"$1/hyprland.conf\" && printf '%s' \"$4\" > \"$1/alacritty.toml\"",
      "chroma-theme",
      root.themeDir,
      gen.files["colors.toml"],
      gen.files["hyprland.conf"],
      gen.files["alacritty.toml"]
    ]
    writeThemeProc.running = true
  }

  function snapshotTheme() {
    try {
      themeNameFile.reload()
    } catch (e) {}
    var name = ""
    try {
      name = String(themeNameFile.text() || "").trim()
    } catch (e2) {}
    ThemeSession.beginPreview(name, root.themeLive)
    root.originalTheme = ThemeSession.snapshot().original
    sessionFile.setText(ThemeSession.serialize())
  }

  function clearThemeSession() {
    ThemeSession.afterSuccessfulRevert()
    root.originalTheme = ""
    root.themeLive = false
    sessionFile.setText(ThemeSession.serialize())
  }

  function applyPreview() {
    if (!ThemeSession.hasRevertTarget()) {
      root.toast = "no revert target — refusing preview"
      ThemeSession.markApplyFailed()
      root.themeLive = false
      return
    }
    root.themeApplyName = "chroma-preview"
    themeSetProc.running = false
    themeSetProc.command = ["omarchy-theme-set", "chroma-preview"]
    themeSetProc.running = true
  }

  function onThemeValidated(ok) {
    root.awaitingThemeValid = false
    if (!ok) {
      root.toast = "chroma-preview failed validation"
      return
    }
    if (!ThemeSession.hasRevertTarget()) {
      root.toast = "no revert target — refusing preview"
      return
    }
    root.applyPreview()
  }

  function revertTheme() {
    var name = root.originalTheme
    if (!name) {
      try {
        var raw = JSON.parse(sessionFile.text() || "{}")
        name = raw.original || ""
      } catch (e) {}
    }
    if (!name || name === "chroma-preview") {
      root.toast = "nothing to revert"
      return
    }
    root.themeApplyName = name
    themeSetProc.running = false
    themeSetProc.command = ["omarchy-theme-set", name]
    themeSetProc.running = true
  }

  function statusJson() {
    return JSON.stringify({
      opened: root.opened,
      backend: client.backend,
      pickMode: client.pickMode,
      themeLive: root.themeLive,
      originalTheme: root.originalTheme
    })
  }

  ChromadClient {
    id: client
    pluginDir: root.pluginDir
    onPicked: function(msg) { root.onPicked(msg) }
    onPaletted: function(msg) {
      root.showAgain()
      root.paletteBusy = false
      root.paletteColors = msg.colors || []
      root.paletteOpen = true
    }
    onOcrText: function(text) {
      root.showAgain()
      root.ocrText = text
      root.hint = text && text.length ? text : "no text"
      root.toast = "OCR done"
    }
    onQrText: function(text) {
      root.showAgain()
      root.qrText = text
      root.hint = text
      if (text)
        root.copyProcText(text)
      root.toast = "QR copied"
    }
    onFailed: function(err) {
      root.showAgain()
      root.paletteBusy = false
      if (root.awaitingThemeValid) {
        root.awaitingThemeValid = false
        root.toast = "theme validation failed: " + err
        return
      }
      root.toast = err
    }
    onThemeValidated: function(ok) { root.onThemeValidated(ok) }
    onFrame: function(msg) {
      if (root.capturing)
        root.showAgain()
      if (msg && msg.pixel)
        root.pixel = msg.pixel
    }
    onHello: function(msg) {
      if (client.pickMode)
        root.hint = "pick mode — Space captures a still"
      client.startStream()
    }
  }

  function copyProcText(text) {
    copyProc.command = ["sh", "-c", "printf %s \"$1\" | wl-copy", "chroma-copy", text]
    copyProc.running = true
  }

  Timer {
    id: hideAckRetry
    interval: 16
    repeat: false
    onTriggered: {
      hideAckProc.running = false
      hideAckProc.running = true
    }
  }

  Process {
    id: hideAckProc
    running: false
    command: ["hyprctl", "-j", "layers"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        if (!root.pendingAction.length || !root.capturing)
          return
        if (root.chromaStillMapped(text) && root.hideAckTries < 24) {
          root.hideAckTries += 1
          hideAckRetry.restart()
          return
        }
        root.runPending()
      }
    }
    onExited: function(code) {
      if (code !== 0 && root.pendingAction.length && root.capturing)
        root.runPending()
    }
  }

  Timer {
    id: themeApplyTimer
    interval: 80
    repeat: false
    onTriggered: {
      if (!client.ready) {
        root.toast = "chromad not ready — theme not applied"
        return
      }
      root.awaitingThemeValid = true
      client.validateTheme(root.themeDir)
    }
  }

  Timer {
    id: cursorTimer
    interval: 16
    running: root.opened && !root.capturing
    repeat: true
    onTriggered: {
      if (!cursorProc.running)
        cursorProc.running = true
    }
  }

  Process {
    id: cursorProc
    running: false
    command: ["hyprctl", "-j", "cursorpos"]
    stdout: StdioCollector {
      waitForEnd: true
      onStreamFinished: {
        var c = Capture.parseCursorText(text)
        if (!c)
          return
        root.cursorX = c.x
        root.cursorY = c.y
        if (client.ready)
          client.sendCursor(c.x, c.y)
        if (client.pickMode && client.ready && !client.live && root.opened && !root.capturing) {
          if (!Capture.frameStillCovers(c.x, c.y, client.frameRect, 24) && client.frameUrl.length === 0)
            return
        }
      }
    }
  }

  Process {
    id: copyProc
    running: false
  }

  Process {
    id: mkdirProc
    running: false
    command: ["mkdir", "-p", root.stateDir]
    onExited: function() {
      historyFile.reload()
      sessionFile.reload()
    }
  }

  Process {
    id: writeThemeProc
    running: false
    onExited: function(code) {
      if (code === 0)
        themeApplyTimer.restart()
      else
        root.toast = "could not write chroma-preview"
    }
  }

  Process {
    id: themeSetProc
    running: false
    stderr: StdioCollector {
      waitForEnd: true
      onStreamFinished: { root.themeSetErr = String(text || "").trim() }
    }
    onExited: function(code) {
      var name = root.themeApplyName
      var err = root.themeSetErr
      root.themeApplyName = ""
      root.themeSetErr = ""
      if (name === "chroma-preview") {
        if (code === 0) {
          ThemeSession.markApplied()
          root.themeLive = true
          sessionFile.setText(ThemeSession.serialize())
          root.toast = "chroma-preview applied — u to revert"
        } else {
          ThemeSession.markApplyFailed()
          root.themeLive = false
          root.toast = err.length ? err : ("omarchy-theme-set failed (" + code + ")")
        }
        return
      }
      if (name && name !== "chroma-preview") {
        if (code === 0) {
          root.clearThemeSession()
          root.toast = "reverted to " + name
        } else {
          root.toast = err.length ? err : ("revert failed (" + code + ")")
        }
      }
    }
  }

  FileView {
    id: historyFile
    path: root.historyPath
    atomicWrites: true
    printErrors: false
    watchChanges: false
    onLoaded: { History.load(text()); root.historyRev = History.revision }
    onLoadFailed: { History.load("{}"); root.historyRev = History.revision }
  }

  FileView {
    id: sessionFile
    path: root.sessionPath
    atomicWrites: true
    printErrors: false
    onLoaded: {
      ThemeSession.load(text() || "{}")
      root.originalTheme = ThemeSession.snapshot().original
      if (ThemeSession.snapshot().live)
        root.themeLive = true
    }
    onLoadFailed: {
      ThemeSession.reset()
      root.originalTheme = ""
    }
  }

  FileView {
    id: themeNameFile
    path: root.themeNamePath
    atomicWrites: false
    printErrors: false
    watchChanges: true
    onFileChanged: reload()
  }

  IpcHandler {
    target: "io.github.chris.chroma"
    function open(payload: string): string { root.open(payload || "{}"); return "ok" }
    function close(arg: string): string { root.close(); return "ok" }
    function toggle(payload: string): string { root.toggle(payload || "{}"); return "ok" }
    function ping(arg: string): string { return "ok" }
    function pick(arg: string): string { return root.pick(arg || "") }
    function palette(arg: string): string { return root.palette(arg || "") }
    function revert(arg: string): string { return root.revert(arg || "") }
    function status(arg: string): string { return root.status(arg || "") }
  }

  PanelWindow {
    id: panel
    visible: root.opened && !root.capturing
    anchors { top: true; bottom: true; left: true; right: true }
    color: "transparent"
    focusable: true
    WlrLayershell.namespace: "chroma"
    WlrLayershell.layer: WlrLayer.Overlay
    WlrLayershell.keyboardFocus: WlrKeyboardFocus.Exclusive
    exclusionMode: ExclusionMode.Ignore
    onVisibleChanged: {
      if (!visible && root.capturing && root.pendingAction.length)
        root.ackOverlayHidden()
    }

    Item {
      id: keyCatcher
      anchors.fill: parent
      focus: true
      Keys.priority: Keys.BeforeItem
      Keys.onPressed: function(event) {
        if (event.key === Qt.Key_Escape) {
          if (root.helpOpen) root.helpOpen = false
          else if (root.paletteOpen) root.paletteOpen = false
          else if (root.rulerOpen) { root.rulerOpen = false; ruler.reset() }
          else if (root.freezeOpen) { root.freezeOpen = false; client.unfreeze() }
          else root.close()
          event.accepted = true
        } else if (event.key === Qt.Key_Space) {
          root.withHiddenCapture("pick")
          event.accepted = true
        } else if (event.key === Qt.Key_C) {
          root.copyHex(true)
          event.accepted = true
        } else if (event.key === Qt.Key_P) {
          root.withHiddenCapture("palette")
          event.accepted = true
        } else if (event.key === Qt.Key_T) {
          root.makeTheme()
          event.accepted = true
        } else if (event.key === Qt.Key_U || event.key === Qt.Key_Backspace) {
          root.revertTheme()
          event.accepted = true
        } else if (event.key === Qt.Key_R) {
          root.rulerOpen = !root.rulerOpen
          if (!root.rulerOpen)
            ruler.reset()
          event.accepted = true
        } else if (event.key === Qt.Key_Z) {
          root.freezeOpen = !root.freezeOpen
          root.zoom = root.zoomDefault
          if (root.freezeOpen)
            root.withHiddenCapture("freeze")
          else
            client.unfreeze()
          event.accepted = true
        } else if (event.key === Qt.Key_Plus || event.key === Qt.Key_Equal) {
          root.zoom = Math.min(16, root.zoom + 1)
          event.accepted = true
        } else if (event.key === Qt.Key_Minus) {
          root.zoom = Math.max(2, root.zoom - 1)
          event.accepted = true
        } else if (event.key === Qt.Key_O) {
          if (client.ocrAvailable)
            root.withHiddenCapture("ocr")
          else
            root.hint = "OCR hidden until tesseract is installed (pacman -S tesseract)"
          event.accepted = true
        } else if (event.key === Qt.Key_Q) {
          if (client.qrAvailable)
            root.withHiddenCapture("qr")
          else
            root.hint = "QR hidden until zbarimg is installed (pacman -S zbar)"
          event.accepted = true
        } else if (event.key === Qt.Key_H) {
          root.historyOpen = !root.historyOpen
          event.accepted = true
        } else if (event.key === Qt.Key_Question || (event.key === Qt.Key_Slash && (event.modifiers & Qt.ShiftModifier))) {
          root.helpOpen = !root.helpOpen
          event.accepted = true
        } else if (event.key === Qt.Key_M) {
          root.withHiddenCapture("palette-monitor")
          event.accepted = true
        }
      }
    }

    MouseArea {
      anchors.fill: parent
      hoverEnabled: true
      acceptedButtons: Qt.LeftButton | Qt.RightButton
      onPositionChanged: function(mouse) {
        if (root.rulerOpen && ruler.start)
          ruler.dragTo(mouse.x, mouse.y)
      }
      onPressed: function(mouse) {
        if (root.rulerOpen) {
          ruler.beginAt(mouse.x, mouse.y)
          mouse.accepted = true
          return
        }
        if (mouse.button === Qt.RightButton) {
          root.historyOpen = true
          mouse.accepted = true
        }
      }
      onReleased: function(mouse) {
        if (root.rulerOpen)
          ruler.dragTo(mouse.x, mouse.y)
      }
      onClicked: function(mouse) {
        if (root.rulerOpen)
          return
        if (mouse.button === Qt.LeftButton)
          root.withHiddenCapture("pick")
      }
    }

    Loupe {
      id: loupe
      frameUrl: client.frameUrl
      frameRect: client.frameRect
      cursorX: root.cursorX
      cursorY: root.cursorY
      zoom: root.zoom
      freezeMode: root.freezeOpen
      accent: root.accent
      offset: root.loupeOffset
      screenW: panel.width
      screenH: panel.height
      pickMode: client.pickMode
      opacity: root.opened ? 1 : 0
      scale: root.opened ? 1 : 0.96
      Behavior on opacity { NumberAnimation { duration: root.motionMs } }
      Behavior on scale { NumberAnimation { duration: root.motionMs } }
    }

    RulerLayer {
      id: ruler
      active: root.rulerOpen
      accent: root.accent
      foreground: root.foreground
      fontFamily: root.fontFamily
    }

    Hud {
      id: hud
      anchors.horizontalCenter: parent.horizontalCenter
      states: [
        State {
          name: "top"
          when: Capture.hudAtTop(root.cursorY, panel.height)
          AnchorChanges { target: hud; anchors.top: panel.top }
          PropertyChanges {
            target: hud
            anchors.topMargin: Style.gapsOut + Style.space(18)
            anchors.bottomMargin: 0
          }
        },
        State {
          name: "bottom"
          when: !Capture.hudAtTop(root.cursorY, panel.height)
          AnchorChanges { target: hud; anchors.bottom: panel.bottom }
          PropertyChanges {
            target: hud
            anchors.bottomMargin: Style.gapsOut + Style.space(18)
            anchors.topMargin: 0
          }
        }
      ]
      pixel: root.pixel
      hexA: root.hexA
      hexB: root.hexB
      pickMode: client.pickMode
      themeLive: root.themeLive
      hint: root.hint
      picks: root.historyOpen ? root.picks : root.picks.slice(0, 8)
      foreground: root.foreground
      background: root.background
      surfaceBorderSpec: root.borderSpec
      fontFamily: root.fontFamily
      opacity: root.opened ? 1 : 0
      onCopyRequested: root.copyHex(true)
      onPaletteRequested: root.withHiddenCapture("palette")
      onThemeRequested: root.makeTheme()
      onRevertRequested: root.revertTheme()
      onHistoryChosen: function(hex) {
        var d = ColorMath.parseColor(hex)
        if (d) {
          root.pixel = d
          root.copyHex(true)
        }
      }
      Behavior on opacity { NumberAnimation { duration: root.motionMs } }
    }

    PaletteFan {
      visible: root.paletteOpen
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.verticalCenter: parent.verticalCenter
      colors: root.paletteColors
      busy: root.paletteBusy
      foreground: root.foreground
      background: root.background
      borderSpec: root.borderSpec
      fontFamily: root.fontFamily
      onMakeTheme: root.makeTheme()
      onSwatchPicked: function(hex) {
        var d = ColorMath.parseColor(hex)
        if (d) {
          root.pixel = d
          History.push(d, root.historyLimit)
          root.historyRev = History.revision
          historyFile.setText(History.serialize())
          root.copyHex(true)
        }
      }
    }

    HelpCard {
      visible: root.helpOpen
      anchors.centerIn: parent
      foreground: root.foreground
      background: root.background
      surfaceBorderSpec: root.borderSpec
      fontFamily: root.fontFamily
      ocrAvailable: client.ocrAvailable
      qrAvailable: client.qrAvailable
    }

    Toast {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.top: parent.top
      anchors.topMargin: Style.gapsOut + Style.space(12)
      message: root.toast
      foreground: root.foreground
      background: root.background
      surfaceBorderSpec: root.borderSpec
      motionMs: root.motionMs
    }
  }

  Component.onCompleted: {
    mkdirProc.running = true
    historyFile.reload()
    sessionFile.reload()
  }
}
