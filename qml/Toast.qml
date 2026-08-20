import QtQuick
import qs.Commons
import qs.Ui

BorderSurface {
  id: root
  property string message: ""
  property color foreground: Color.menu.text
  property color background: Color.menu.background
  property var surfaceBorderSpec: Border.surfaceSpec("menu", "border", Color.menu.border, 1)
  property int motionMs: 150
  property bool dismissed: false

  visible: message.length > 0 && !dismissed
  opacity: visible ? 1 : 0
  implicitWidth: Math.min(label.implicitWidth + Style.space(24), 420)
  implicitHeight: label.implicitHeight + Style.space(16)
  radius: Style.cornerRadius
  color: background
  borderSpec: root.surfaceBorderSpec

  Behavior on opacity { NumberAnimation { duration: root.motionMs } }

  Text {
    id: label
    anchors.centerIn: parent
    text: root.message
    color: root.foreground
    font.family: Style.font.menuFamily
    font.pixelSize: Style.font.body
  }

  Timer {
    id: hide
    interval: 1400
    onTriggered: root.dismissed = true
  }

  onMessageChanged: {
    root.dismissed = false
    if (message.length)
      hide.restart()
  }
}
