import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { delimiter, join } from 'node:path';
import test from 'node:test';
import { fileURLToPath } from 'node:url';

// Exercise the real shell script with a fake NetworkManager; no root or network access.
const script = fileURLToPath(new URL('../src-tauri/scripts/wg.sh', import.meta.url));
const uuid = '12345678-1234-1234-1234-123456789abc';
const nmcli = `#!${process.execPath}
const { appendFileSync } = require('node:fs');
const args = process.argv.slice(2);
appendFileSync(process.env.CALL_LOG, args.join(' ') + '\\n');
const scenario = process.env.SCENARIO;
const command = args.join(' ');
if (command === '--version') {
  console.log('nmcli 1.46');
} else if (command === '-t -f NAME connection show --active') {
  if (scenario === 'query-fails') process.exit(1);
  if (scenario === 'active' || scenario === 'down-fails') console.log('wg-gui-home');
} else if (command === '-t -f NAME connection show') {
  console.log('home'); // An unrelated user's connection must survive.
} else if (args[0] === 'connection' && args[1] === 'down' && scenario === 'down-fails') {
  console.error('not authorized');
  process.exit(1);
} else if (args[0] === 'connection' && args[1] === 'import') {
  console.log("Connection 'home' (${uuid}) successfully added.");
} else if (args[0] === 'connection' && args[1] === 'modify' && scenario === 'rename-fails') {
  process.exit(1);
}
`;

function runScript(scenario: string, action: string) {
  const root = mkdtempSync(join(tmpdir(), 'wg-script-test-'));
  try {
    writeFileSync(join(root, 'nmcli'), nmcli, { mode: 0o700 });
    const profiles = join(root, '.config/wireguard-gui/profiles');
    mkdirSync(profiles, { recursive: true });
    writeFileSync(join(profiles, 'home.conf'), '[Interface]\nPrivateKey = secret\n');
    const log = join(root, 'calls');
    const result = spawnSync('bash', [script], {
      env: {
        ...process.env,
        HOME: root,
        PROFILE: 'home',
        WG_ACTION: action,
        IS_SNAP: 'true',
        SCENARIO: scenario,
        CALL_LOG: log,
        PATH: `${root}${delimiter}${process.env.PATH ?? ''}`,
      },
      encoding: 'utf8',
      timeout: 10_000,
    });
    assert.ifError(result.error);
    assert.equal(result.signal, null, 'WireGuard script terminated by a signal');
    return { ...result, calls: readFileSync(log, 'utf8') };
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

test('reports disconnect failures without importing a connection', () => {
  const result = runScript('down-fails', 'disconnect');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Failed to disconnect/);
  assert.doesNotMatch(result.calls, /connection import/);
});

test('connecting an active connection is idempotent', () => {
  const result = runScript('active', 'connect');
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.calls, /connection down/);
  assert.doesNotMatch(result.calls, /connection import/);
});

test('connecting preserves unmanaged connections and targets the imported UUID', () => {
  const result = runScript('inactive', 'connect');
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.calls, /connection delete home/);
  assert.ok(result.calls.includes(`connection modify uuid ${uuid} connection.id wg-gui-home`));
  assert.ok(result.calls.includes(`connection up uuid ${uuid}`));
});

test('a failed active query is not reported as disconnected', () => {
  const result = runScript('query-fails', 'disconnect');
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /Failed to query active/);
  assert.doesNotMatch(result.calls, /connection down/);
});

test('a rename failure stops before activation', () => {
  const result = runScript('rename-fails', 'connect');
  assert.notEqual(result.status, 0);
  assert.doesNotMatch(result.calls, /connection up/);
});
