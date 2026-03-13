#!/bin/bash
set -euo pipefail

home="${HOME:-}"
profile="${PROFILE:-}"
is_snap="${IS_SNAP:-false}"

if [[ -z "$profile" ]]; then
  echo "PROFILE is required" >&2
  exit 2
fi

# Linux interface names are max 15 chars.
if [[ ! "$profile" =~ ^[a-zA-Z0-9_.=-]{1,15}$ ]]; then
  echo "Invalid PROFILE/interface name: $profile" >&2
  exit 2
fi

echo "Connecting to $profile (IS_SNAP=$is_snap)"

user_conf="$home/.config/wireguard-gui/profiles/$profile.conf"
profile_path="/etc/wireguard/$profile.conf"

if [[ ! -f "$user_conf" ]]; then
  echo "Profile not found: $user_conf" >&2
  exit 1
fi

if [[ "$is_snap" == "true" ]]; then
  # Snap mode: use NetworkManager, not raw ip/wg netlink operations.
  if ! command -v nmcli >/dev/null 2>&1; then
    echo "nmcli not found in snap runtime" >&2
    exit 1
  fi

  conn_name="wg-gui-${profile}"

  if nmcli -t -f NAME connection show --active | grep -Fxq "$conn_name"; then
    nmcli connection down "$conn_name"
    echo "Connection $conn_name brought down (SNAP mode)"
    exit 0
  fi

  if ! nmcli -t -f NAME connection show | grep -Fxq "$conn_name"; then
    import_output="$(nmcli connection import type wireguard file "$user_conf" 2>&1)" || {
      echo "$import_output" >&2
      exit 1
    }

    imported_name="$(printf '%s\n' "$import_output" | sed -n "s/.*Connection '\([^']\+\)'.*/\1/p" | tail -n 1)"
    if [[ -n "$imported_name" && "$imported_name" != "$conn_name" ]]; then
      nmcli connection modify "$imported_name" connection.id "$conn_name"
    elif [[ -z "$imported_name" ]]; then
      if nmcli -t -f NAME connection show | grep -Fxq "$profile"; then
        nmcli connection modify "$profile" connection.id "$conn_name"
      fi
    fi
  fi

  nmcli connection up "$conn_name"
  echo "Connection $conn_name brought up (SNAP mode)"
  exit 0
fi

# Normal mode: keep existing pkexec + wg-quick behavior.
tmp_script="$(mktemp)"
cat <<EOF > "$tmp_script"
#!/bin/bash
set -euo pipefail

cp -f "$user_conf" "$profile_path"

if ip link show dev "$profile" >/dev/null 2>&1; then
  wg-quick down "$profile"
else
  wg-quick up "$profile"
fi
EOF

chmod +x "$tmp_script"
pkexec "$tmp_script"
status=$?

echo "Return code: $status"

rm -f "$tmp_script"
exit "$status"
