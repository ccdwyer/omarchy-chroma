import QtQuick
import qs.Commons
import qs.Ui

BorderSurface {
  id: root
  property var pixel: null
  property string hexA: ""
  property string hexB: ""
  property bool pickMode: false
  property bool themeLive: false
  property bool ocrAvailable: false
  property bool qrAvailable: false
  property string hint: ""
  property color foreground: Color.menu.text
  property color background: Color.menu.background
  property var borderSpec: Border.surfaceSpec("menu", "border", Color.menu.border, 1)
  property string fontFamily: Style.font.menuFamily
  property var picks: []

  signal copyRequested()
  signal paletteRequested()
  signal themeRequested()
  signal revertRequested()
  signal historyChosen(string hex)

  width: Math.min(Style.space(640), parent ? parent.width - Style.space(48) : 640)
  height: col.implicitHeight + Style.space(24)
  radius: Style.cornerRadius
  color: background
  borderSpec: root.borderSpec

  Column {
    id: col
    anchors.left: parent.left
    anchors.right: parent.right
    anchors.top: parent.top
    anchors.margins: Style.space(14)
    spacing: Style.space(8)

    Row {
      spacing: Style.space(10)
      Rectangle {
        width: Style.space(28)
        height: Style.space(28)
        radius: 6
        color: pixel && pixel.hex ? pixel.hex : "transparent"
        border.color: root.foreground
        border.width: 1
      }
      Column {
        spacing: 2
        Text {
          text: pixel && pixel.hex ? pixel.hex : "—"
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.heading
          font.bold: true
        }
        Text {
          text: pixel ? (pixel.oklch || "") : ""
          color: root.foreground
          opacity: 0.7
          font.family: root.fontFamily
          font.pixelSize: Style.font.bodySmall
        }
      }
      ContrastBadge {
        hexA: root.hexA
        hexB: root.hexB
        foreground: root.foreground
        fontFamily: root.fontFamily
        anchors.verticalCenter: parent.verticalCenter
      }
    }

    Text {
      width: col.width
      visible: pixel !== null
      text: pixel ? ((pixel.rgb || "") + "  ·  " + (pixel.hsl || "")) : ""
      color: root.foreground
      opacity: 0.75
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      wrapMode: Text.WordWrap
    }

    HistoryStrip {
      width: col.width
      picks: root.picks
      foreground: root.foreground
      fontFamily: root.fontFamily
      onChosen: function(hex) { root.historyChosen(hex) }
    }

    Text {
      visible: root.pickMode
      text: "pick mode — Space captures a still (grim fallback, not live)"
      color: Color.accent
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
    }

    Text {
      visible: root.hint.length > 0
      text: root.hint
      color: root.foreground
      opacity: 0.7
      font.family: root.fontFamily
      font.pixelSize: Style.font.bodySmall
      wrapMode: Text.WordWrap
      width: col.width
    }

    Row {
      spacing: Style.space(8)
      Repeater {
        model: [
          { id: "copy", label: "copy  c" },
          { id: "palette", label: "palette  p" },
          { id: "theme", label: "theme  t" },
          { id: "revert", label: "revert  u" }
        ]
        delegate: Rectangle {
          required property var modelData
          visible: modelData.id !== "revert" || root.themeLive
          height: Style.space(26)
          width: txt.implicitWidth + Style.space(16)
          radius: height / 2
          color: modelData.id === "theme" || modelData.id === "revert" ? Color.accent : Qt.rgba(1, 1, 1, 0.08)
          Text {
            id: txt
            anchors.centerIn: parent
            text: modelData.label
            color: modelData.id === "theme" || modelData.id === "revert" ? root.background : root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.bodySmall
          }
          MouseArea {
            anchors.fill: parent
            onClicked: {
              if (modelData.id === "copy") root.copyRequested()
              else if (modelData.id === "palette") root.paletteRequested()
              else if (modelData.id === "theme") root.themeRequested()
              else if (modelData.id === "revert") root.revertRequested()
            }
          }
        }
      }
    }
  }
}
