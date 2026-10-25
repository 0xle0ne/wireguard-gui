"""Exercise connection management with a fake NetworkManager; no root/network access."""
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "scripts" / "wg.sh"

NMCLI = '''#!/usr/bin/env python3
import os, sys
from pathlib import Path
args = sys.argv[1:]
with open(os.environ["CALL_LOG"], "a") as log:
    log.write(" ".join(args) + "\\n")
scenario = os.environ["SCENARIO"]
if args == ["--version"]:
    print("nmcli 1.46")
elif args == ["-t", "-f", "NAME", "connection", "show", "--active"]:
    if scenario == "query-fails":
        sys.exit(1)
    if scenario in ["active", "down-fails"]:
        print("wg-gui-home")
elif args == ["-t", "-f", "NAME", "connection", "show"]:
    print("home")  # An unrelated user's connection must survive.
elif args[:2] == ["connection", "down"] and scenario == "down-fails":
    print("not authorized", file=sys.stderr)
    sys.exit(1)
elif args[:2] == ["connection", "import"]:
    print("Connection 'home' (12345678-1234-1234-1234-123456789abc) successfully added.")
elif args[:2] == ["connection", "modify"] and scenario == "rename-fails":
    sys.exit(1)
'''


class ConnectionScriptTests(unittest.TestCase):
    def run_script(self, scenario, action):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "nmcli"
            binary.write_text(NMCLI)
            binary.chmod(0o700)
            profiles = root / ".config/wireguard-gui/profiles"
            profiles.mkdir(parents=True)
            (profiles / "home.conf").write_text("[Interface]\nPrivateKey = secret\n")
            log = root / "calls"
            env = dict(os.environ, HOME=str(root), PROFILE="home", WG_ACTION=action,
                       IS_SNAP="true", SCENARIO=scenario, CALL_LOG=str(log),
                       PATH=str(root) + os.pathsep + os.environ["PATH"])
            result = subprocess.run(["bash", str(SCRIPT)], env=env,
                                    capture_output=True, text=True, timeout=10)
            return result, log.read_text()

    def test_disconnect_failure_is_reported(self):
        result, calls = self.run_script("down-fails", "disconnect")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Failed to disconnect", result.stderr)
        self.assertNotIn("connection import", calls)

    def test_connect_is_idempotent_for_active_connection(self):
        result, calls = self.run_script("active", "connect")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("connection down", calls)
        self.assertNotIn("connection import", calls)

    def test_connect_preserves_unmanaged_connection(self):
        result, calls = self.run_script("inactive", "connect")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn("connection delete home", calls)
        self.assertIn("connection modify uuid 12345678-1234-1234-1234-123456789abc connection.id wg-gui-home", calls)
        self.assertIn("connection up uuid 12345678-1234-1234-1234-123456789abc", calls)

    def test_failed_active_query_is_not_reported_as_disconnected(self):
        result, calls = self.run_script("query-fails", "disconnect")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Failed to query active", result.stderr)
        self.assertNotIn("connection down", calls)

    def test_rename_failure_stops_before_activation(self):
        result, calls = self.run_script("rename-fails", "connect")
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("connection up", calls)


if __name__ == "__main__":
    unittest.main()
