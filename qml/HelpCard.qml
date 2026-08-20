import QtQuick
import qs.Commons
import qs.Ui

BorderSurface {
  id: root
  property color foreground: Color.menu.text
  property color background: Color.menu.background
  property var surfaceBorderSpec: Border.surfaceSpec("menu", "border", Color.menu.border, 1)
  property string fontFamily: Style.font.menuFamily
  property bool ocrAvailable: false
  property bool qrAvailable: false
  property bool offerBinds: false
  property string offerNote: ""

  signal keysRequested()

  width: Style.space(420)
  height: col.implicitHeight + Style.space(32)
  radius: Style.cornerRadius
  color: background
  borderSpec: root.surfaceBorderSpec

  readonly property var rows: {
    var list = [
      ["Esc", "close lens"],
      ["Space", "pick color"],
      ["c", "copy HEX"],
      ["p", "palette from window"],
      ["t", "make theme"],
      ["u", "revert theme"],
      ["r", "pixel ruler"],
      ["z / + / −", "freeze zoom"],
      ["h", "pick history"],
      ["?", "this help"]
    ]
    if (ocrAvailable)
      list.push(["o", "OCR (tesseract)"])
    else
      list.push(["", "OCR: pacman -S tesseract"])
    if (qrAvailable)
      list.push(["q", "QR (zbarimg)"])
    else
      list.push(["", "QR: pacman -S zbar"])
    return list
  }

  Column {
    id: col
    anchors.fill: parent
    anchors.margins: Style.space(16)
    spacing: Style.space(8)

    Text {
      text: "Chroma"
      color: root.foreground
      font.family: root.fontFamily
      font.pixelSize: Style.font.heading
      font.bold: true
    }

    Repeater {
      model: root.rows
      delegate: Row {
        width: col.width
        spacing: Style.space(12)
        Text {
          width: Style.space(110)
          text: modelData[0]
          color: Color.accent
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
          font.bold: true
        }
        Text {
          text: modelData[1]
          color: root.foreground
          opacity: 0.8
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
        }
      }
    }

    Text {
      visible: root.offerBinds
      width: col.width
      text: root.offerNote.length ? root.offerNote : "Add Super+Alt+C (skips combos you already use)"
      color: root.foreground
      opacity: 0.8
      wrapMode: Text.WordWrap
      font.family: root.fontFamily
      font.pixelSize: Style.font.body
    }

    Rectangle {
      visible: root.offerBinds
      width: bindLabel.implicitWidth + Style.space(16)
      height: bindLabel.implicitHeight + Style.space(10)
      radius: Math.max(4, Style.cornerRadius / 2)
      color: Color.accent
      Text {
        id: bindLabel
        anchors.centerIn: parent
        text: "Add keybindings"
        color: root.background
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
      }
      MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.keysRequested()
      }
    }
  }
}
