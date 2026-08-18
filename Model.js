function emptyReport() {
  return { error: "", online: 0, total: 0, servers: [] }
}

function parseReport(raw) {
  try {
    var data = JSON.parse(String(raw || ""))
    if (!data || typeof data !== "object") return emptyReport()
    var servers = []
    var listed = data.servers
    if (listed && listed.length) {
      for (var i = 0; i < listed.length; i++) {
        var s = listed[i]
        if (!s) continue
        servers.push({
          name: String(s.name || "Server"),
          host: String(s.host || ""),
          is_online: s.is_online === true,
          open_ports: Array.isArray(s.open_ports) ? s.open_ports : [],
          timestamp: String(s.timestamp || "")
        })
      }
    }
    return {
      error: typeof data.error === "string" ? data.error : "",
      online: Number(data.online || 0),
      total: Number(data.total || servers.length || 0),
      servers: servers
    }
  } catch (e) {
    return { error: "Invalid network_checker output", online: 0, total: 0, servers: [] }
  }
}

function portsText(server) {
  if (!server) return ""
  if (!server.is_online) return "unreachable"
  var ports = server.open_ports || []
  if (ports.length === 0) return "ping only"
  return "ports " + ports.join(", ")
}

function defaultCommand() {
  return ""
}

function defaultConfigFile() {
  return ""
}

if (typeof module !== "undefined") {
  module.exports = {
    emptyReport: emptyReport,
    parseReport: parseReport,
    portsText: portsText,
    defaultCommand: defaultCommand,
    defaultConfigFile: defaultConfigFile
  }
}
