import QtQuick
import qs.Commons
import qs.Ui
import "../js/Capture.js" as Capture

Item {
  id: root
  property var frameRect: ({ x: 0, y: 0, w: 128, h: 128 })
  property string frameUrl: ""
  property real cursorX: 0
  property real cursorY: 0
  property int zoom: 4
  property bool freezeMode: false
  property color accent: Color.accent
  property color chrome: "#ff2bd6"
  property int offset: 60
  property int screenW: 1920
  property int screenH: 1080
  property bool pickMode: false

  readonly property int viewPx: freezeMode ? Math.max(16, Math.round(128 / Math.max(1, zoom))) : 48
  readonly property int size: 192
  readonly property var clip: Capture.sourceClip(cursorX, cursorY, frameRect, viewPx)
  readonly property var pos: Capture.loupePosition(cursorX, cursorY, size, size, screenW, screenH, offset)

  x: pos.x
  y: pos.y
  width: size
  height: size

  Rectangle {
    anchors.fill: parent
    radius: width / 2
    color: Qt.rgba(0, 0, 0, 0.35)
    border.width: 3
    border.color: root.accent
    clip: true

    Image {
      id: img
      anchors.fill: parent
      anchors.margins: 4
      source: root.frameUrl
      cache: false
      smooth: false
      asynchronous: true
      fillMode: Image.PreserveAspectCrop
      sourceClipRect: Qt.rect(root.clip.x, root.clip.y, root.clip.w, root.clip.h)
      visible: root.frameUrl.length > 0 && status === Image.Ready
    }

    Rectangle {
      visible: !img.visible
      anchors.fill: parent
      anchors.margins: 4
      radius: width / 2
      color: Qt.rgba(0, 0, 0, 0.55)
      Text {
        anchors.centerIn: parent
        text: root.pickMode ? "pick" : "…"
        color: "white"
        font.pixelSize: Style.font.body
      }
    }

    Rectangle {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.verticalCenter: parent.verticalCenter
      width: 10
      height: 1
      color: root.accent
    }
    Rectangle {
      anchors.horizontalCenter: parent.horizontalCenter
      anchors.verticalCenter: parent.verticalCenter
      width: 1
      height: 10
      color: root.accent
    }
  }

  Text {
    anchors.horizontalCenter: parent.horizontalCenter
    anchors.top: parent.bottom
    anchors.topMargin: 4
    text: root.freezeMode ? (root.zoom + "×") : (root.pickMode ? "pick mode" : "")
    color: root.accent
    font.pixelSize: Style.font.bodySmall
    visible: text.length > 0
  }
}
