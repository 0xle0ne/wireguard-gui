#!/bin/bash
# Manual WireGuard Client Config Generator

set -e

CLIENT_NAME="${1:-fedora}"
SERVER_HOST="miraclemax.local"
SERVER_USER="jbyrd"

echo "🔑 Creating WireGuard client config for: $CLIENT_NAME"
echo ""

# Generate client keys locally
echo "Generating client keys..."
CLIENT_PRIVATE_KEY=$(wg genkey)
CLIENT_PUBLIC_KEY=$(echo "$CLIENT_PRIVATE_KEY" | wg pubkey)

echo "Client public key: $CLIENT_PUBLIC_KEY"
echo ""

# Get server info
echo "Getting server information from MiracleMax..."
SERVER_PUBLIC_KEY=$(ssh $SERVER_USER@$SERVER_HOST "sudo cat /etc/wireguard/wg0_privatekey | wg pubkey" 2>/dev/null || \
                   ssh $SERVER_USER@$SERVER_HOST "sudo grep PrivateKey /etc/wireguard/wg0.conf | cut -d' ' -f3 | wg pubkey")

SERVER_ENDPOINT=$(ssh $SERVER_USER@$SERVER_HOST "hostname -I | awk '{print \$1}'"):51820

# Find next available IP
LAST_IP=$(ssh $SERVER_USER@$SERVER_HOST "sudo grep 'AllowedIPs' /etc/wireguard/wg0.conf | grep -oP '10\.8\.0\.\K\d+' | sort -n | tail -1" 2>/dev/null || echo "1")
NEXT_IP=$((LAST_IP + 1))
CLIENT_IP="10.8.0.$NEXT_IP"

echo "Server endpoint: $SERVER_ENDPOINT"
echo "Client IP: $CLIENT_IP/24"
echo ""

# Create client config
mkdir -p ~/.config/wireguard-gui/profiles

cat > ~/.config/wireguard-gui/profiles/miraclemax.conf <<EOF
[Interface]
PrivateKey = $CLIENT_PRIVATE_KEY
Address = $CLIENT_IP/24
DNS = 1.1.1.1, 8.8.8.8

[Peer]
PublicKey = $SERVER_PUBLIC_KEY
Endpoint = $SERVER_ENDPOINT
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25
EOF

echo "✅ Client config created: ~/.config/wireguard-gui/profiles/miraclemax.conf"
echo ""

# Add peer to server
echo "Adding peer to server..."
ssh $SERVER_USER@$SERVER_HOST "sudo bash -c 'cat >> /etc/wireguard/wg0.conf <<PEER

[Peer]
# $CLIENT_NAME
PublicKey = $CLIENT_PUBLIC_KEY
AllowedIPs = $CLIENT_IP/32
PEER
'"

# Reload WireGuard on server
echo "Reloading WireGuard on server..."
ssh $SERVER_USER@$SERVER_HOST "sudo systemctl reload wg-quick@wg0 || sudo wg syncconf wg0 <(wg-quick strip wg0)"

echo ""
echo "✅ Setup complete!"
echo ""
echo "Client configuration:"
echo "────────────────────────────────────────────────────"
cat ~/.config/wireguard-gui/profiles/miraclemax.conf
echo "────────────────────────────────────────────────────"
echo ""
echo "📋 Next steps:"
echo "  1. Open WireGuard GUI"
echo "  2. Click '+' to add profile"
echo "  3. Name: miraclemax"
echo "  4. Copy/paste the config above"
echo "  5. Click rocket icon to connect"
echo ""

