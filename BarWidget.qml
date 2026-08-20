import QtQuick
import Quickshell
import qs.Commons
import qs.Ui
import "js/History.js" as History
import "js/Color.js" as ColorMath
import "js/Binds.js" as Binds

BarWidget {
  id: root
  moduleName: "io.github.chris.chroma"

  property int loupeOffset: 60
  property int historyLimit: 24
  property int zoomDefault: 4
  property string lastHex: ""
  property bool offerBinds: false
  property string offerNote: ""

  readonly property string pluginId: "io.github.chris.chroma"

  function open() { root.summonOverlay(root.payloadJson()) }
  function close() { root.hideOverlay() }
  function toggle() { root.toggleOverlay(root.payloadJson()) }

  function summonOverlay(payload) {
    var body = payload || root.payloadJson()
    if (bar && bar.shell && typeof bar.shell.summon === "function") {
      bar.shell.summon(root.pluginId, body)
      return
    }
    Quickshell.execDetached(["omarchy-shell", "shell", "summon", root.pluginId, body])
  }

  function hideOverlay() {
    if (bar && bar.shell && typeof bar.shell.hide === "function") {
      bar.shell.hide(root.pluginId)
      return
    }
    Quickshell.execDetached(["omarchy-shell", "shell", "hide", root.pluginId])
  }

  function toggleOverlay(payload) {
    var body = payload || root.payloadJson()
    if (bar && bar.shell && typeof bar.shell.toggle === "function") {
      bar.shell.toggle(root.pluginId, body)
      return
    }
    Quickshell.execDetached(["omarchy-shell", "shell", "toggle", root.pluginId, body])
  }

  function payloadJson() {
    return JSON.stringify({
      loupeOffset: root.loupeOffset,
      historyLimit: root.historyLimit,
      zoomDefault: root.zoomDefault
    })
  }

  function refresh() {
    var snap = History.snapshot()
    root.lastHex = snap.picks.length ? snap.picks[0].hex : ""
    var offer = Binds.offer || {}
    root.offerBinds = !!offer.needed
    root.offerNote = String(offer.note || "Add Super+Alt+C")
  }

  function installBinds() {
    Quickshell.execDetached(["omarchy-shell", root.pluginId, "installBinds", ""])
  }

  implicitWidth: row.implicitWidth
  implicitHeight: row.implicitHeight

  Timer {
    interval: 800
    running: true
    repeat: true
    onTriggered: root.refresh()
  }

  Row {
    id: row
    spacing: Style.space(4)

    WidgetButton {
      id: button
      bar: root.bar
      text: "●"
      tooltipText: root.lastHex
                   ? ("Chroma " + root.lastHex + " — click lens, right-click history")
                   : "Chroma — screen lens (click) · history (right-click)"
      onPressed: function(buttonCode) {
        if (buttonCode === Qt.LeftButton)
          root.toggleOverlay(root.payloadJson())
        else if (buttonCode === Qt.RightButton)
          root.summonOverlay(JSON.stringify({
            loupeOffset: root.loupeOffset,
            historyLimit: root.historyLimit,
            zoomDefault: root.zoomDefault,
            mode: "history"
          }))
      }

      Rectangle {
        visible: ColorMath.normalizeHex(root.lastHex) !== null
        width: Style.space(8)
        height: Style.space(8)
        radius: width / 2
        anchors.right: parent.right
        anchors.top: parent.top
        color: root.lastHex || Color.accent
        border.width: 1
        border.color: Color.menu.border
      }
    }

    WidgetButton {
      visible: root.offerBinds
      bar: root.bar
      text: "keys"
      tooltipText: root.offerNote.length ? root.offerNote : "Add Super+Alt+C keybinding (skips combos you already use)"
      onPressed: function(buttonCode) {
        if (buttonCode === Qt.LeftButton)
          root.installBinds()
      }
    }
  }

  Component.onCompleted: root.refresh()
}
