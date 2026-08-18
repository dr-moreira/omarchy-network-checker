# daniel.network-checker

Omarchy / Quickshell bar widget for [network_checker](https://github.com). Shows home-server ping and port status on the menubar.

## Requirements

- [Omarchy](https://omarchy.org/) with `omarchy-shell`
- `network_checker` on `~/.local/bin/network_checker` (or set the widget `command` setting)
- Servers listed in `~/.config/network_checker/config.toml`

`network_checker` must support `--json`.

## Install

From this folder (local checkout):

```bash
ln -sfn "$PWD" ~/.config/omarchy/plugins/daniel.network-checker
omarchy-shell shell rescanPlugins
omarchy plugin enable daniel.network-checker --section right
```

From git once published:

```bash
omarchy plugin add <git-url> --enable --yes
```

## Usage

- Left click: open server list
- Middle / right click: refresh
- Click a row: copy host to clipboard
- `r` in the popup: refresh

## Config

Widget settings live in `~/.config/omarchy/shell.json` on the `daniel.network-checker` bar entry:

- `refreshIntervalSec` — poll interval (default 60)
- `command` — optional path to `network_checker`
