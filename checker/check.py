#!/usr/bin/env python3

import argparse
import json
import os
import socket
import subprocess
import sys
import tomllib
from datetime import datetime, timezone
from pathlib import Path
from typing import NoReturn


def emit(payload, code=0) -> NoReturn:
    print(json.dumps(payload, separators=(",", ":")))
    raise SystemExit(code)


def error(message) -> NoReturn:
    emit({"error": message, "online": 0, "total": 0, "servers": []}, 1)


def find_config(custom) -> Path:
    if custom:
        path = Path(custom).expanduser()
        if path.is_file():
            return path
        error(f"Config not found: {path}")

    names = ("network_checker.toml", "config.toml")
    for name in names:
        path = Path(name)
        if path.is_file():
            return path

    config_home = Path(os.environ.get("XDG_CONFIG_HOME", Path.home() / ".config"))
    for name in names:
        path = config_home / "network_checker" / name
        if path.is_file():
            return path

    error(
        "No servers configured. Copy checker/config.example.toml to "
        "~/.config/network_checker/config.toml"
    )


def ping(host):
    result = subprocess.run(
        ["ping", "-c", "1", "-W", "1", host],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return result.returncode == 0


def port_open(host, port, timeout):
    try:
        with socket.create_connection((host, port), timeout=timeout):
            return True
    except OSError:
        return False


def main():
    parser = argparse.ArgumentParser(description="Ping and port check for Network Checker")
    parser.add_argument("-f", "--file", help="TOML config path")
    parser.add_argument("-j", "--json", action="store_true", help="JSON report")
    args = parser.parse_args()
    if not args.json:
        args.json = True

    path = find_config(args.file)
    try:
        data = tomllib.loads(path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as exc:
        error(str(exc))

    servers = data.get("servers") or []
    if not servers:
        error("No servers configured in the config file")

    settings = data.get("settings") or {}
    timeout_ms = settings.get("port_timeout_ms", 2000)
    try:
        timeout = max(0.2, float(timeout_ms) / 1000.0)
    except (TypeError, ValueError):
        timeout = 2.0

    now = datetime.now(timezone.utc).astimezone().isoformat()
    report = []
    for server in servers:
        name = str(server.get("name") or "Server")
        host = str(server.get("host") or "")
        ports = server.get("ports") or []
        online = bool(host) and ping(host)
        open_ports = []
        if online:
            for port in ports:
                try:
                    port_n = int(port)
                except (TypeError, ValueError):
                    continue
                if port_open(host, port_n, timeout):
                    open_ports.append(port_n)
        report.append({
            "name": name,
            "host": host,
            "is_online": online,
            "open_ports": open_ports,
            "timestamp": now,
        })

    emit({
        "error": "",
        "online": sum(1 for item in report if item["is_online"]),
        "total": len(report),
        "servers": report,
    })


if __name__ == "__main__":
    try:
        main()
    except BrokenPipeError:
        sys.exit(0)
