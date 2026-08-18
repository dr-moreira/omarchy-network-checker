import QtQuick
import QtQuick.Controls
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Ui
import "Model.js" as Model

Panel {
  id: root
  moduleName: "io.github.dr-moreira.network-checker"
  manageIpc: false

  property var anchorItem: null
  property var hostWidget: null

  property var servers: []
  property int onlineCount: 0
  property int totalCount: 0
  property string lastError: ""
  property bool refreshing: false
  property bool hasReport: false
  property int selectedIndex: 0
  property bool cursorActive: false
  property string copiedHost: ""

  readonly property int refreshIntervalSec: {
    var n = parseInt(String(setting("refreshIntervalSec", 60)), 10)
    if (!isFinite(n) || n < 15) n = 60
    if (n > 3600) n = 3600
    return n
  }
  readonly property string configuredCommand: String(setting("command", "") || "").trim()
  readonly property string configuredConfig: String(setting("configFile", "") || "").trim()
  readonly property string bundledChecker: {
    var url = String(Qt.resolvedUrl("checker/check.py"))
    return url.indexOf("file://") === 0 ? url.slice(7) : url
  }
  readonly property color foreground: bar ? bar.foreground : Color.foreground
  readonly property color urgent: bar ? bar.urgent : Color.urgent
  readonly property color dim: Qt.darker(foreground, 1.55)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  readonly property bool allOnline: hasReport && totalCount > 0 && onlineCount === totalCount && lastError === ""
  readonly property bool anyOffline: hasReport && totalCount > 0 && onlineCount < totalCount
  readonly property string barLabel: {
    if (root.vertical) return "󰒍"
    if (!hasReport && refreshing) return "󰒍 …"
    if (lastError !== "" && totalCount === 0) return "󰒍 ?"
    if (totalCount > 0) return "󰒍 " + onlineCount + "/" + totalCount
    return "󰒍"
  }
  readonly property string heroMeta: {
    if (refreshing && !hasReport) return "Checking servers"
    if (lastError !== "" && totalCount === 0) return lastError
    if (totalCount === 0) return "No servers configured"
    if (allOnline) return "All " + totalCount + " servers online"
    return onlineCount + " of " + totalCount + " online"
  }
  readonly property color hoverFill: bar ? Style.hoverFillFor(bar.foreground, Color.accent) : "transparent"
  readonly property color selectedFill: bar ? Style.selectedFillFor(bar.foreground, Color.accent) : "transparent"

  function open() {
    root.controller.show()
  }

  function close() {
    root.controller.hide()
  }

  function switchPanel(direction) {
    if (root.bar && typeof root.bar.switchPanelFrom === "function")
      return root.bar.switchPanelFrom(root.hostWidget || root, direction)
    return false
  }

  function refresh() {
    if (checkProc.running) return
    refreshing = true
    var cmd
    if (configuredCommand !== "") cmd = [configuredCommand, "--json"]
    else cmd = ["python3", bundledChecker, "--json"]
    if (configuredConfig !== "") cmd.push("--file", configuredConfig)
    checkProc.command = cmd
    checkProc.running = true
  }

  function applyReport(raw, exitCode) {
    var parsed = Model.parseReport(raw)
    if (parsed.error !== "" && parsed.total === 0) {
      lastError = parsed.error
      if (!hasReport) {
        servers = []
        onlineCount = 0
        totalCount = 0
      }
      return
    }
    servers = parsed.servers
    onlineCount = parsed.online
    totalCount = parsed.total
    lastError = parsed.error !== "" ? parsed.error : (exitCode !== 0 ? "Check failed" : "")
    hasReport = true
    if (selectedIndex >= servers.length) selectedIndex = Math.max(0, servers.length - 1)
  }

  function selectedServer() {
    if (servers.length === 0) return null
    return servers[Math.max(0, Math.min(selectedIndex, servers.length - 1))]
  }

  function setServerCursor(index) {
    cursorActive = true
    selectedIndex = index
    scrollCursorIntoView()
  }

  function moveCursor(dy) {
    cursorActive = true
    if (servers.length === 0) return
    selectedIndex = Math.max(0, Math.min(servers.length - 1, selectedIndex + dy))
    scrollCursorIntoView()
  }

  function scrollCursorIntoView() {
    if (!serverColumn || selectedIndex < 0 || selectedIndex >= serverColumn.children.length) return
    var item = serverColumn.children[selectedIndex]
    if (!panelFlick || !item) return
    Qt.callLater(function() {
      if (!item) return
      var margin = Style.space(6)
      var point = item.mapToItem(panelFlick.contentItem, 0, 0)
      var top = point.y
      var bottom = top + item.height
      var viewTop = panelFlick.contentY
      var viewBottom = viewTop + panelFlick.height
      var maxY = Math.max(0, panelFlick.contentHeight - panelFlick.height)
      if (top < viewTop + margin) panelFlick.contentY = Math.max(0, top - margin)
      else if (bottom > viewBottom - margin) panelFlick.contentY = Math.min(maxY, bottom + margin - panelFlick.height)
    })
  }

  function copyHost(server) {
    if (!server || !server.host) return
    Quickshell.execDetached(["wl-copy", String(server.host)])
    copiedHost = String(server.host)
    copiedTimer.restart()
  }

  onOpenedChanged: if (opened) {
    cursorActive = false
    if (panelFlick) panelFlick.contentY = 0
    refresh()
    Qt.callLater(function() { keyCatcher.forceActiveFocus() })
  }

  KeyboardPanel {
    id: panel
    anchorItem: root.anchorItem
    owner: root.hostWidget || root
    bar: root.bar
    open: root.opened
    focusTarget: keyCatcher
    contentWidth: panel.fittedContentWidth(Style.space(380))
    contentHeight: panel.fittedContentHeight(column.implicitHeight, Style.space(560))

    PanelKeyCatcher {
      id: keyCatcher
      anchors.fill: parent
      onMoveRequested: function(dx, dy) {
        if (!root.cursorActive) { root.cursorActive = true; return }
        root.moveCursor(dy)
      }
      onActivateRequested: if (root.cursorActive) root.copyHost(root.selectedServer())
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(t) {
        if (t === "r" || t === "R") root.refresh()
      }

      Flickable {
        id: panelFlick
        anchors.fill: parent
        contentWidth: width
        contentHeight: column.implicitHeight
        clip: true
        boundsBehavior: Flickable.StopAtBounds
        flickableDirection: Flickable.VerticalFlick
        interactive: contentHeight > height
        ScrollBar.vertical: ScrollBar { policy: ScrollBar.AsNeeded }

        Column {
          id: column
          width: panelFlick.width
          spacing: Style.space(12)

          PanelHero {
            id: hero
            width: parent.width
            title: "Network Checker"
            meta: root.heroMeta
            foreground: root.foreground
            fontFamily: root.fontFamily
            iconOpacity: root.allOnline ? 1.0 : 0.7
            iconComponent: Component {
              Text {
                text: "󰒍"
                color: root.anyOffline ? root.urgent : root.foreground
                font.family: root.fontFamily
                font.pixelSize: Style.font.display
                horizontalAlignment: Text.AlignHCenter
                verticalAlignment: Text.AlignVCenter
              }
            }
          }

          Text {
            visible: root.lastError !== ""
            width: parent.width
            text: root.lastError
            color: root.urgent
            font.family: root.fontFamily
            font.pixelSize: Style.font.bodySmall
            wrapMode: Text.WordWrap
          }

          PanelSeparator {
            visible: root.servers.length > 0
            foreground: root.foreground
          }

          Column {
            visible: root.servers.length > 0
            width: parent.width
            spacing: Style.space(10)

            PanelSectionHeader {
              text: "SERVERS"
              foreground: root.foreground
              fontFamily: root.fontFamily
            }

            Column {
              id: serverColumn
              width: parent.width
              spacing: Style.space(6)

              Repeater {
                model: root.servers
                ServerRow {
                  required property var modelData
                  required property int index
                  width: serverColumn.width
                  server: modelData
                  rowIndex: index
                }
              }
            }
          }
        }
      }
    }
  }

  Process {
    id: checkProc
    running: false
    command: []
    stdout: StdioCollector { id: checkStdout; waitForEnd: true }
    stderr: StdioCollector { id: checkStderr; waitForEnd: true }
    onExited: function(exitCode) {
      root.refreshing = false
      var stdout = String(checkStdout.text || "").trim()
      var stderr = String(checkStderr.text || "").trim()
      if (stdout !== "") root.applyReport(stdout, exitCode)
      else root.lastError = stderr !== "" ? stderr : "network_checker failed"
    }
  }

  Timer {
    interval: root.refreshIntervalSec * 1000
    running: true
    repeat: true
    triggeredOnStart: true
    onTriggered: root.refresh()
  }

  Timer {
    id: copiedTimer
    interval: 1800
    onTriggered: root.copiedHost = ""
  }

  component ServerRow: CursorSurface {
    id: serverRow
    property var server: null
    property int rowIndex: 0
    readonly property bool online: server && server.is_online === true
    readonly property string name: server ? String(server.name || "Server") : "Server"
    readonly property string host: server ? String(server.host || "") : ""
    readonly property string detail: {
      var ports = Model.portsText(server)
      if (root.copiedHost !== "" && root.copiedHost === host) return "copied " + host
      return host + (ports !== "" ? " · " + ports : "")
    }

    hasCursor: root.cursorActive && root.selectedIndex === rowIndex
    current: !online
    foreground: root.foreground
    fill: root.hoverFill
    currentFill: root.selectedFill

    implicitHeight: row.implicitHeight + Style.spacing.lg

    Row {
      id: row
      anchors.left: parent.left
      anchors.right: parent.right
      anchors.verticalCenter: parent.verticalCenter
      anchors.leftMargin: Style.space(6)
      anchors.rightMargin: Style.space(6)
      spacing: Style.space(8)

      Text {
        text: serverRow.online ? "●" : "○"
        color: serverRow.online ? root.foreground : root.urgent
        font.family: root.fontFamily
        font.pixelSize: Style.font.body
        width: Style.space(22)
        horizontalAlignment: Text.AlignHCenter
        anchors.verticalCenter: parent.verticalCenter
      }

      Column {
        width: parent.width - Style.space(30)
        anchors.verticalCenter: parent.verticalCenter
        spacing: Style.space(1)

        Text {
          width: parent.width
          text: serverRow.name
          color: root.foreground
          font.family: root.fontFamily
          font.pixelSize: Style.font.body
          font.bold: !serverRow.online
          elide: Text.ElideRight
        }

        Text {
          width: parent.width
          text: serverRow.detail
          color: root.dim
          font.family: root.fontFamily
          font.pixelSize: Style.font.caption
          elide: Text.ElideRight
        }
      }
    }

    MouseArea {
      id: rowMouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onEntered: root.setServerCursor(serverRow.rowIndex)
      onClicked: root.copyHost(serverRow.server)
    }

    PanelToolTip {
      visible: rowMouse.containsMouse
      text: "Copy " + serverRow.host
      fontFamily: root.fontFamily
    }
  }
}
