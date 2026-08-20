import QtQuick
import qs.Commons
import qs.Ui

Item {
  id: root
  property var picks: []
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily
  signal chosen(string hex)

  height: Style.space(36)
  width: parent ? parent.width : 200

  Text {
    anchors.verticalCenter: parent.verticalCenter
    text: picks.length ? "" : "no picks yet — Space to sample"
    color: root.foreground
    opacity: 0.55
    font.family: root.fontFamily
    font.pixelSize: Style.font.bodySmall
    visible: picks.length === 0
  }

  ListView {
    anchors.fill: parent
    orientation: ListView.Horizontal
    spacing: Style.space(8)
    clip: true
    model: root.picks
    visible: picks.length > 0
    delegate: Rectangle {
      required property var modelData
      width: Style.space(28)
      height: Style.space(28)
      radius: 6
      color: modelData.hex
      border.color: root.foreground
      border.width: 1
      MouseArea {
        anchors.fill: parent
        onClicked: root.chosen(modelData.hex)
      }
    }
  }
}
