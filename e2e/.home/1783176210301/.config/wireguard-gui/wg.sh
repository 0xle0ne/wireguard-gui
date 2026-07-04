#!/bin/bash
set -euo pipefail

home="${HOME:-}"
profile="${PROFILE:-}"
is_snap="${IS_SNAP:-false}"

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

# Linux interface names are max 15 chars.
if [[ ! "$profile" =~ ^[a-zA-Z0-9_.=-]{1,15}$ ]]; then
  fail "Invalid PROFILE/interface name: $profile" 2
fi

echo "Connecting to $profile (IS_SNAP=$is_snap)"

user_conf="$home/.config/wireguard-gui/profiles/$profile.conf"
profile_path="/etc/wireguard/$profile.conf"

if [[ ! -f "$user_conf" ]]; then
  fail "Profile not found: $user_conf" 3
fi

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

  # Check if connection is active and disconnect it
  if run_with_timeout 10 nmcli -t -f NAME connection show --active 2>/dev/null | grep -Fxq "$conn_name"; then
    echo "Disconnecting $conn_name..."
    run_with_timeout 15 nmcli connection down "$conn_name" 2>&1 || {
      echo "Warning: Failed to disconnect $conn_name" >&2
    }
    echo "Connection $conn_name brought down"
    exit 0
  fi

  # Clean up any old connections with the same name (handles renames/updates)
  if run_with_timeout 10 nmcli -t -f NAME connection show 2>/dev/null | grep -Fxq "$conn_name"; then
    echo "Removing old connection $conn_name..."
    run_with_timeout 10 nmcli connection delete "$conn_name" 2>&1 || {
      echo "Warning: Failed to delete old connection $conn_name" >&2
    }
  fi

  # Also clean up connection with the profile name
  if run_with_timeout 10 nmcli -t -f NAME connection show 2>/dev/null | grep -Fxq "$profile"; then
    echo "Removing old connection $profile..."
    run_with_timeout 10 nmcli connection delete "$profile" 2>&1 || {
      echo "Warning: Failed to delete old connection $profile" >&2
    }
  fi

  # Import the connection
  echo "Importing connection from $user_conf..."
  import_output=""
  if ! import_output=$(run_with_timeout 20 nmcli connection import type wireguard file "$user_conf" 2>&1); then
    echo "Failed to import connection: $import_output" >&2
    exit 10
  fi

  # Extract the imported connection name
  imported_name=$(printf '%s\n' "$import_output" | sed -n "s/.*Connection '\([^']*\)'.*/\1/p" | head -n 1)

  if [[ -z "$imported_name" ]]; then
    echo "Failed to extract imported connection name from: $import_output" >&2
    exit 10
  fi

  # Rename to our standard naming convention if needed
  if [[ "$imported_name" != "$conn_name" ]]; then
    echo "Renaming connection from $imported_name to $conn_name..."
    if ! run_with_timeout 10 nmcli connection modify "$imported_name" connection.id "$conn_name" 2>&1; then
      echo "Warning: Failed to rename connection" >&2
    fi
  fi

  # Keep app-managed connections manual; do not auto-connect on boot.
  if ! run_with_timeout 10 nmcli connection modify "$conn_name" connection.autoconnect no 2>&1; then
    echo "Warning: Failed to disable autoconnect for $conn_name" >&2
  fi

  # Bring up the connection
  echo "Bringing up connection $conn_name..."
  if ! run_with_timeout 20 nmcli connection up "$conn_name" 2>&1; then
    echo "Failed to bring up connection $conn_name" >&2
    exit 11
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

if [[ -z "$profile" || -z "$user_conf" || -z "$profile_path" ]]; then
  echo "Error: Invalid parameters" >&2
  exit 1
fi

# Copy profile to system wireguard directory
cp -f "$user_conf" "$profile_path"

# Toggle the connection
if ip link show dev "$profile" >/dev/null 2>&1; then
  # Interface exists, bring it down
  echo "Bringing down interface $profile..."
  wg-quick down "$profile"
else
  # Interface doesn't exist, bring it up
  echo "Bringing up interface $profile..."
  wg-quick up "$profile"
fi
EOF

chmod +x "$tmp_script"

# Run as root using pkexec with proper error handling
status=0
pkexec "$tmp_script" "$profile" "$user_conf" "$profile_path" || status=$?
if [[ "$status" -ne 0 ]]; then
  echo "wg-quick failed with status $status" >&2
  exit "$status"
fi

echo "wg-quick completed successfully"
exit 0
