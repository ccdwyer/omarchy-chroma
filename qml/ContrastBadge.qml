import QtQuick
import qs.Commons
import "../js/Color.js" as ColorMath

Rectangle {
  id: root
  property string hexA: ""
  property string hexB: ""
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily

  readonly property var info: (hexA && hexB) ? ColorMath.contrastInfo(hexA, hexB) : null
  readonly property bool pass: info ? info.aa : false

  visible: info !== null
  implicitWidth: label.implicitWidth + Style.space(16)
  implicitHeight: label.implicitHeight + Style.space(8)
  radius: height / 2
  color: pass ? Qt.rgba(0.15, 0.45, 0.22, 0.85) : Qt.rgba(0.45, 0.12, 0.12, 0.85)
  border.color: pass ? "#7dcea0" : "#e74c3c"
  border.width: 1

  Text {
    id: label
    anchors.centerIn: parent
    color: "white"
    font.family: root.fontFamily
    font.pixelSize: Style.font.bodySmall
    text: info ? (info.ratioText + " " + info.rating + (info.aa ? " ✓" : " ✗")) : ""
  }
}
