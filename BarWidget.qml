import QtQuick
import Quickshell
import qs.Commons
import qs.Ui
import "js/History.js" as History
import "js/Color.js" as ColorMath

BarWidget {
  id: root
  moduleName: "io.github.chris.chroma"

  property int loupeOffset: 60
  property int historyLimit: 24
  property int zoomDefault: 4
  property string lastHex: ""

  readonly property string pluginId: "io.github.chris.chroma"

  function open() { root.summonOverlay("{}") }
  function close() {}
  function toggle() { root.summonOverlay("{}") }

  function summonOverlay(payload) {
    var body = payload || root.payloadJson()
    if (bar && bar.shell && typeof bar.shell.summon === "function") {
      bar.shell.summon(root.pluginId, body)
      return
    }
    if (bar && bar.shell && typeof bar.shell.toggle === "function") {
      bar.shell.toggle(root.pluginId, body)
      return
    }
    Quickshell.execDetached(["omarchy-shell", "shell", "summon", root.pluginId, body])
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
  }

  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  Timer {
    interval: 800
    running: true
    repeat: true
    onTriggered: root.refresh()
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    text: "●"
    tooltipText: root.lastHex
                 ? ("Chroma " + root.lastHex + " — click lens, right-click history")
                 : "Chroma — screen lens (click) · history (right-click)"
    onPressed: function(buttonCode) {
      if (buttonCode === Qt.LeftButton)
        root.summonOverlay(root.payloadJson())
      else if (buttonCode === Qt.RightButton)
        root.summonOverlay(JSON.stringify({
          loupeOffset: root.loupeOffset,
          historyLimit: root.historyLimit,
          zoomDefault: root.zoomDefault,
          mode: "history"
        }))
    }
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

  Component.onCompleted: root.refresh()
}
