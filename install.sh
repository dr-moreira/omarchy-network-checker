#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "$0")" && pwd)"
bin_dir="${HOME}/.local/bin"
config_dir="${HOME}/.config/network_checker"
bin_path="${bin_dir}/network_checker"

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required. Install rustup, then run this script again." >&2
  exit 1
fi

cargo build --release --manifest-path "${root}/checker/Cargo.toml"
mkdir -p "${bin_dir}" "${config_dir}"
install -Dm755 "${root}/checker/target/release/network_checker" "${bin_path}"

if [[ ! -f "${config_dir}/config.toml" ]]; then
  cp "${root}/checker/config.example.toml" "${config_dir}/config.toml"
  echo "Wrote ${config_dir}/config.toml — edit your hosts and ports."
else
  echo "Kept existing ${config_dir}/config.toml"
fi

echo "Installed ${bin_path}"
