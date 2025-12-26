#!/bin/bash

home=$HOME
profile=$PROFILE

echo "Connecting to $profile"

# Detect if running in snap environment
if [ -n "$SNAP" ]; then
    # Running in snap - use snap-compatible approach
    echo "Snap environment detected"
    
    # Use SNAP_DATA for configuration (writable area in snap)
    profile_path="$SNAP_DATA/wireguard/$profile.conf"
    config_dir="$SNAP_DATA/wireguard"
    
    # Ensure config directory exists
    mkdir -p "$config_dir"
    
    # Copy profile to snap data directory
    cp -f "$home/.config/wireguard-gui/profiles/$profile.conf" "$profile_path"
    
    # In snap, network-control interface provides necessary privileges
    # No need for pkexec - commands run with required capabilities
    if ip a | grep -q "$profile"; then
        wg-quick down "$profile"
        STATUS=$?
    else
        wg-quick up "$profile"
        STATUS=$?
    fi
    
    echo "Return code: $STATUS"
    exit $STATUS
    
else
    # Running outside snap - use traditional pkexec approach
    echo "Standard environment detected"
    
    profile_path="/etc/wireguard/$profile.conf"
    
    # Create a temporary script under /tmp/wireguard-tmp.sh
    cat <<EOF > /tmp/wireguard-tmp.sh
#!/bin/bash

cp -f "$home/.config/wireguard-gui/profiles/$profile.conf" "$profile_path"

if ip a | grep -q $profile; then
  wg-quick down $profile
else
  wg-quick up $profile
fi
EOF

    chmod +x /tmp/wireguard-tmp.sh
    
    pkexec /tmp/wireguard-tmp.sh
    
    STATUS=$?
    
    echo "Return code: $STATUS"
    
    rm /tmp/wireguard-tmp.sh
    
    exit $STATUS
fi
