# Network Checker

Omarchy bar widget that pings configured hosts and checks ports via the bundled `network_checker` Rust CLI. Offline servers highlight on the bar.

## Requirements

- [Omarchy](https://omarchy.org/) with `omarchy-shell` (Quattro)
- Rust/`cargo` to build the checker
- `wl-copy` to copy a host from the panel
- `iputils` (`ping`)

## Install

```sh
omarchy plugin add https://github.com/dr-moreira/omarchy-network-checker.git --enable
PLUGIN="$HOME/.config/omarchy/plugins/io.github.dr-moreira.network-checker"
cargo build --release --manifest-path "$PLUGIN/checker/Cargo.toml"
mkdir -p "$HOME/.local/bin"
cp "$PLUGIN/checker/target/release/network_checker" "$HOME/.local/bin/network_checker"
mkdir -p "$HOME/.config/network_checker"
test -f "$HOME/.config/network_checker/config.toml" || \
  cp "$PLUGIN/checker/config.example.toml" "$HOME/.config/network_checker/config.toml"
```

Edit `~/.config/network_checker/config.toml` with your hosts and ports. The example is copied only when that file does not exist.

## Usage

- Left click: open or close the server list
- Middle / right click: refresh
- Click a row: copy the host with `wl-copy`
- `r` in the popup: refresh
- Escape: close

```sh
omarchy-shell shell summon io.github.dr-moreira.network-checker '{}'
omarchy-shell shell hide io.github.dr-moreira.network-checker
```

## Configure

```sh
omarchy bar move io.github.dr-moreira.network-checker --section right
```

Widget settings on the bar entry in `~/.config/omarchy/shell.json`:

- `refreshIntervalSec` — poll interval (default 60)
- `command` — optional path to `network_checker`
- `configFile` — optional TOML path passed as `--file`

The checker also reads `~/.config/network_checker/config.toml` by default.

```toml
[[servers]]
name = "NAS"
host = "192.168.1.10"
ports = [22, 445]
```

Standalone CLI:

```sh
network_checker --json
network_checker --daemon
network_checker --file /path/to/config.toml
```

## Remove

```sh
omarchy plugin remove io.github.dr-moreira.network-checker
rm -f ~/.local/bin/network_checker
```

`~/.config/network_checker/config.toml` is left in place.

## License

MIT
