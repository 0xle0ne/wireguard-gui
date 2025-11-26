#!/bin/bash
# End-to-End Test: Deploy WireGuard + Test GUI with MiracleMax

set -e

BOLD="\033[1m"
GREEN="\033[32m"
YELLOW="\033[33m"
RESET="\033[0m"

echo -e "${BOLD}${GREEN}🔐 End-to-End WireGuard Test${RESET}"
echo "════════════════════════════════════════════════════"
echo ""
echo "This will:"
echo "  1. Check/deploy WireGuard server on MiracleMax"
echo "  2. Generate client config for this laptop"
echo "  3. Import into GUI"
echo "  4. Test connection"
echo ""

# Check if MiracleMax is reachable
echo -e "${BOLD}Step 1: Checking MiracleMax connection...${RESET}"
if ! ping -c 1 -W 2 miraclemax.local &>/dev/null; then
    echo -e "${YELLOW}⚠️  Cannot reach miraclemax.local${RESET}"
    echo "Make sure MiracleMax is on the network"
    exit 1
fi
echo -e "${GREEN}✅ MiracleMax is reachable${RESET}"
echo ""

# Check if WireGuard is deployed on MiracleMax
echo -e "${BOLD}Step 2: Checking WireGuard server status...${RESET}"
if ssh jbyrd@miraclemax.local "systemctl is-active wg-quick@wg0" &>/dev/null; then
    echo -e "${GREEN}✅ WireGuard server is running on MiracleMax${RESET}"
    SERVER_RUNNING=true
else
    echo -e "${YELLOW}⚠️  WireGuard not running on MiracleMax${RESET}"
    echo ""
    echo "Would you like to deploy it now? (requires Ansible)"
    read -p "Deploy WireGuard to MiracleMax? (y/N): " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cd /home/jbyrd/ansai/orchestrators/ansible
        echo "Deploying WireGuard..."
        ./deploy-wireguard-to-miraclemax.sh
        SERVER_RUNNING=true
    else
        echo "Skipping deployment. You'll need to deploy manually first."
        SERVER_RUNNING=false
    fi
fi
echo ""

if [ "$SERVER_RUNNING" != "true" ]; then
    echo "Please deploy WireGuard to MiracleMax first, then run this test again."
    exit 1
fi

# Generate client config
echo -e "${BOLD}Step 3: Generating client config...${RESET}"

CLIENT_NAME="$(hostname | cut -d. -f1)"
echo "Client name: $CLIENT_NAME"

# Check if client already exists
if ssh jbyrd@miraclemax.local "test -f /etc/wireguard/clients/$CLIENT_NAME.conf"; then
    echo -e "${YELLOW}⚠️  Client '$CLIENT_NAME' already exists on server${RESET}"
    read -p "Regenerate config? (y/N): " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        ssh jbyrd@miraclemax.local "sudo rm -f /etc/wireguard/clients/$CLIENT_NAME.conf"
        ssh jbyrd@miraclemax.local "sudo /usr/local/bin/wireguard-add-client $CLIENT_NAME"
    fi
else
    echo "Creating new client config..."
    ssh jbyrd@miraclemax.local "sudo /usr/local/bin/wireguard-add-client $CLIENT_NAME"
fi

# Download the config
mkdir -p ~/.config/wireguard-gui/profiles
scp jbyrd@miraclemax.local:/etc/wireguard/clients/$CLIENT_NAME.conf ~/.config/wireguard-gui/profiles/miraclemax.conf

echo -e "${GREEN}✅ Client config downloaded to: ~/.config/wireguard-gui/profiles/miraclemax.conf${RESET}"
echo ""

# Show the config
echo -e "${BOLD}Client Configuration:${RESET}"
echo "────────────────────────────────────────────────────"
grep -E "^(Address|DNS|Endpoint|PublicKey)" ~/.config/wireguard-gui/profiles/miraclemax.conf | head -10
echo "────────────────────────────────────────────────────"
echo ""

echo -e "${BOLD}Step 4: Testing with GUI...${RESET}"
echo ""
echo "The WireGuard GUI should now be open on your laptop."
echo ""
echo "📋 Manual test steps:"
echo ""
echo "1. In the GUI, click the '+' button to add a profile"
echo "2. Name it: 'miraclemax'"
echo "3. Paste the config content:"
echo "   cat ~/.config/wireguard-gui/profiles/miraclemax.conf"
echo ""
echo "4. Save the profile"
echo "5. Click the rocket icon to connect"
echo "6. Watch for the animation fix working when disconnected!"
echo ""
echo "Expected results:"
echo "  ✅ Profile imports successfully"
echo "  ✅ Connection establishes"
echo "  ✅ Icon turns green and locked"
echo "  ✅ When disconnected, icon pulses (our CSS fix!)"
echo "  ✅ Shows your public IP"
echo ""

read -p "Press Enter when ready to test connection from terminal..."

# Test connection from terminal
echo ""
echo -e "${BOLD}Testing connection from terminal...${RESET}"

if ip link show wg0 &>/dev/null; then
    echo -e "${GREEN}✅ WireGuard interface wg0 exists${RESET}"
    
    if ping -c 3 -W 2 10.8.0.1 &>/dev/null; then
        echo -e "${GREEN}✅ Can ping MiracleMax through VPN (10.8.0.1)${RESET}"
        echo ""
        echo "🎉 SUCCESS! VPN is working!"
        echo ""
        echo "You can now access MiracleMax services:"
        echo "  • SSH: ssh jbyrd@10.8.0.1"
        echo "  • Story Stages: http://books.jbyrd.org"
        echo "  • PassGo: http://passgo.jbyrd.org"
        echo "  • Actual Budget: http://actual.jbyrd.org"
    else
        echo -e "${YELLOW}⚠️  Cannot ping MiracleMax VPN IP${RESET}"
        echo "Connection may still be establishing..."
    fi
else
    echo -e "${YELLOW}⚠️  No wg0 interface found${RESET}"
    echo "Make sure you connected in the GUI"
fi

echo ""
echo "═══════════════════════════════════════════════════"
echo -e "${BOLD}${GREEN}Testing Complete!${RESET}"
echo "═══════════════════════════════════════════════════"
echo ""
echo "Your fixes are working if you see:"
echo "  ✅ Disconnected icon pulses smoothly (CSS fix)"
echo "  ✅ Proper spacing below icon"
echo "  ✅ Can connect to MiracleMax"
echo "  ✅ Can access services through VPN"
echo ""
echo "Ready to contribute these fixes! 🚀"
echo ""

