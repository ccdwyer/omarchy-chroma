import QtQuick
import qs.Commons
import qs.Ui

Item {
  id: root
  property var colors: []
  property color foreground: Color.menu.text
  property color background: Color.menu.background
  property var borderSpec: Border.surfaceSpec("menu", "border", Color.menu.border, 1)
  property string fontFamily: Style.font.menuFamily
  property bool busy: false
  signal makeTheme()
  signal swatchPicked(string hex)

  width: Style.space(420)
  height: col.implicitHeight + Style.space(24)

  BorderSurface {
    anchors.fill: parent
    radius: Style.cornerRadius
    color: root.background
    borderSpec: root.borderSpec

    Column {
      id: col
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.top: parent.top
      anchors.margins: Style.space(16)
      spacing: Style.space(12)

      Text {
        text: root.busy ? "extracting palette…" : "palette"
        color: root.foreground
        font.family: root.fontFamily
        font.pixelSize: Style.font.heading
        font.bold: true
      }

      Row {
        spacing: Style.space(8)
        Repeater {
          model: root.colors
          delegate: Rectangle {
            required property var modelData
            width: Style.space(48)
            height: Style.space(48)
            radius: 8
            color: modelData
            border.color: root.foreground
            border.width: 1
            MouseArea {
              anchors.fill: parent
              onClicked: root.swatchPicked(modelData)
            }
          }
        }
      }

      Rectangle {
        width: Style.space(140)
        height: Style.space(32)
        radius: height / 2
        color: Color.accent
        enabled: root.colors.length > 0 && !root.busy
        opacity: enabled ? 1 : 0.45
        Text {
          anchors.centerIn: parent
          text: "Make theme"
          color: root.background
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
          font.bold: true
        }
        MouseArea {
          anchors.fill: parent
          enabled: parent.enabled
          onClicked: root.makeTheme()
        }
      }

      Text {
        text: "t make theme  ·  u revert"
        color: root.foreground
        opacity: 0.55
        font.family: root.fontFamily
        font.pixelSize: Style.font.bodySmall
      }
    }
  }
}
