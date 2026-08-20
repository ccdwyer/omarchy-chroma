import QtQuick
import qs.Commons
import "../js/Capture.js" as Capture

Item {
  id: root
  property bool active: false
  property var start: null
  property var end: null
  property color accent: Color.accent
  property color foreground: Color.menu.text
  property string fontFamily: Style.font.menuFamily

  visible: active
  anchors.fill: parent

  readonly property var a: start || { x: 0, y: 0 }
  readonly property var b: end || start || { x: 0, y: 0 }
  readonly property int px: (start && end) ? Capture.rulerDistance(a, b) : 0

  function beginAt(x, y) {
    root.start = { x: x, y: y }
    root.end = { x: x, y: y }
  }

  function dragTo(x, y) {
    if (!root.start)
      root.start = { x: x, y: y }
    root.end = { x: x, y: y }
  }

  function reset() {
    root.start = null
    root.end = null
  }

  Canvas {
    id: canvas
    anchors.fill: parent
    onPaint: {
      var ctx = getContext("2d")
      ctx.clearRect(0, 0, width, height)
      if (!root.start)
        return
      ctx.strokeStyle = Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.8)
      ctx.fillStyle = ctx.strokeStyle
      ctx.lineWidth = 2
      ctx.beginPath()
      ctx.moveTo(root.a.x, root.a.y)
      ctx.lineTo(root.b.x, root.b.y)
      ctx.stroke()
      function handle(pt) {
        ctx.beginPath()
        ctx.arc(pt.x, pt.y, 5, 0, Math.PI * 2)
        ctx.fill()
      }
      handle(root.a)
      handle(root.b)
    }
  }

  onStartChanged: canvas.requestPaint()
  onEndChanged: canvas.requestPaint()
  onActiveChanged: canvas.requestPaint()

  Rectangle {
    visible: root.start && root.end
    x: (root.a.x + root.b.x) / 2 - width / 2
    y: (root.a.y + root.b.y) / 2 - height - 8
    width: label.implicitWidth + Style.space(12)
    height: label.implicitHeight + Style.space(8)
    radius: 6
    color: Qt.rgba(0, 0, 0, 0.7)
    Text {
      id: label
      anchors.centerIn: parent
      text: root.px + " px"
      color: "white"
      font.family: root.fontFamily
      font.pixelSize: Style.font.body
    }
  }
}
