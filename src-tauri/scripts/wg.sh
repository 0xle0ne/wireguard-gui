#!/bin/bash
set -euo pipefail
export LC_ALL=C

home="${HOME:-}"
profile="${PROFILE:-}"
is_snap="${IS_SNAP:-false}"
wg_action="${WG_ACTION:-toggle}"

run_with_timeout() {
  local duration="$1"
  shift
  if command -v timeout >/dev/null 2>&1; then
    timeout "$duration" "$@"
  else
    "$@"
  fi
}

fail() {
  echo "$1" >&2
  exit "${2:-1}"
}

if [[ -z "$home" ]]; then
  fail "HOME is required" 2
fi

if [[ -z "$profile" ]]; then
  echo "PROFILE is required" >&2
  exit 2
fi

if [[ "$wg_action" != "toggle" && "$wg_action" != "connect" && "$wg_action" != "disconnect" ]]; then
  fail "Invalid WG_ACTION: $wg_action" 2
fi

# Linux interface names are max 15 chars.
if [[ ! "$profile" =~ ^[a-zA-Z0-9_.=-]{1,15}$ ]]; then
  fail "Invalid PROFILE/interface name: $profile" 2
fi

echo "Connecting to $profile (IS_SNAP=$is_snap)"

user_conf="$home/.config/wireguard-gui/profiles/$profile.conf"
profile_path="/etc/wireguard/$profile.conf"

# Check if nmcli is available
has_nmcli() {
  command -v nmcli >/dev/null 2>&1 && run_with_timeout 5 nmcli --version >/dev/null 2>&1
}

if [[ "$is_snap" == "true" ]] && ! has_nmcli; then
  fail "nmcli is required in snap mode but was not found" 127
fi

# Use nmcli if available (works in both snap and native environments)
if has_nmcli; then
  echo "NetworkManager detected, using nmcli for connection management"

  conn_name="wg-gui-${profile}"

  # A failed status query must never be treated as a disconnected tunnel.
  active_connections=$(run_with_timeout 10 nmcli -t -f NAME connection show --active) || {
    fail "Failed to query active NetworkManager connections"
  }
  if grep -Fxq -- "$conn_name" <<< "$active_connections"; then
    if [[ "$wg_action" == "connect" ]]; then
      echo "Connection $conn_name already connected"
      exit 0
    fi
    echo "Disconnecting $conn_name..."
    run_with_timeout 15 nmcli connection down "$conn_name" 2>&1 || {
      fail "Failed to disconnect $conn_name"
    }
    echo "Connection $conn_name brought down"
    exit 0
  fi

  if [[ "$wg_action" == "disconnect" ]]; then
    echo "Connection $conn_name already disconnected"
    exit 0
  fi

  if [[ ! -f "$user_conf" ]]; then
    fail "Profile not found: $user_conf" 3
  fi

  # Clean up any old connections with the same name (handles renames/updates)
  existing_connections=$(run_with_timeout 10 nmcli -t -f NAME connection show) || {
    fail "Failed to query NetworkManager connections"
  }
  if grep -Fxq -- "$conn_name" <<< "$existing_connections"; then
    echo "Removing old connection $conn_name..."
    run_with_timeout 10 nmcli connection delete "$conn_name" 2>&1 || {
      fail "Failed to delete old connection $conn_name"
    }
  fi

  # Import the connection
  echo "Importing connection from $user_conf..."
  import_output=""
  if ! import_output=$(run_with_timeout 20 nmcli connection import type wireguard file "$user_conf" 2>&1); then
    echo "Failed to import connection: $import_output" >&2
    exit 10
  fi

  # Connection IDs are not unique. Use the imported UUID so a duplicate
  # user-created connection name can never be modified or activated instead.
  imported_uuid=$(printf '%s\n' "$import_output" | sed -nE 's/.*\(([[:xdigit:]]{8}(-[[:xdigit:]]{4}){3}-[[:xdigit:]]{12})\).*/\1/p' | head -n 1)
  if [[ -z "$imported_uuid" ]]; then
    fail "Failed to extract imported connection UUID from: $import_output" 10
  fi

  echo "Configuring imported connection $imported_uuid as $conn_name..."
  if ! run_with_timeout 10 nmcli connection modify uuid "$imported_uuid" connection.id "$conn_name" connection.autoconnect no 2>&1; then
    fail "Failed to configure imported connection" 10
  fi

  echo "Bringing up connection $conn_name..."
  if ! run_with_timeout 20 nmcli connection up uuid "$imported_uuid" 2>&1; then
    fail "Failed to bring up connection $conn_name" 11
  fi
  echo "Connection $conn_name brought up successfully"
  exit 0
fi

if [[ "$is_snap" == "true" ]]; then
  fail "nmcli is required in snap mode and fallback is disabled" 127
fi

# Normal mode: use pkexec + wg-quick behavior (default fallback)
echo "Using wg-quick mode (not running in snap or nmcli unavailable)"

# Create a temporary script to run as root
tmp_script="$(mktemp)"
trap "rm -f '$tmp_script'" EXIT

cat <<'EOF' > "$tmp_script"
#!/bin/bash
set -euo pipefail

profile="${1:-}"
user_conf="${2:-}"
profile_path="${3:-}"
wg_action="${4:-toggle}"

if [[ -z "$profile" || -z "$profile_path" ]]; then
  echo "Error: Invalid parameters" >&2
  exit 1
fi

# Toggle the connection
if ip link show dev "$profile" >/dev/null 2>&1; then
  if [[ "$wg_action" == "connect" ]]; then
    echo "Interface $profile already connected"
    exit 0
  fi
  # Interface exists, bring it down
  echo "Bringing down interface $profile..."
  wg-quick down "$profile"
else
  if [[ "$wg_action" == "disconnect" ]]; then
    echo "Interface $profile already disconnected"
    exit 0
  fi

  if [[ -z "$user_conf" || ! -f "$user_conf" ]]; then
    echo "Profile not found: $user_conf" >&2
    exit 3
  fi

  # Copy profile to system wireguard directory
  install -d -m 700 /etc/wireguard
  install -m 600 "$user_conf" "$profile_path"

  # Interface doesn't exist, bring it up
  echo "Bringing up interface $profile..."
  wg-quick up "$profile"
fi
EOF

chmod +x "$tmp_script"

# Run as root using pkexec with proper error handling
status=0
pkexec "$tmp_script" "$profile" "$user_conf" "$profile_path" "$wg_action" || status=$?
if [[ "$status" -ne 0 ]]; then
  echo "wg-quick failed with status $status" >&2
  exit "$status"
fi

echo "wg-quick completed successfully"
exit 0
