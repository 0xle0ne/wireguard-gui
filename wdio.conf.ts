import { existsSync, mkdirSync, readFileSync } from 'node:fs';
import path, { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const appPath = path.resolve(__dirname);
const tauriConfigPath = join(appPath, 'src-tauri', 'tauri.conf.json');

if (!existsSync(tauriConfigPath)) {
  throw new Error(`Tauri config not found: ${tauriConfigPath}`);
}

const tauriConfig = JSON.parse(readFileSync(tauriConfigPath, 'utf-8')) as {
  productName?: string;
};
const productName = tauriConfig.productName || 'wireguard-gui';
const binaryName = productName.toLowerCase().replace(/\s+/g, '-');

let appBinaryPath: string;
if (process.platform === 'win32') {
  appBinaryPath = join(
    appPath,
    'src-tauri',
    'target',
    'debug',
    `${binaryName}.exe`,
  );
} else if (process.platform === 'linux') {
  appBinaryPath = join(appPath, 'src-tauri', 'target', 'debug', binaryName);
} else {
  throw new Error(
    `Unsupported platform for this test setup: ${process.platform}`,
  );
}

if (!existsSync(appBinaryPath)) {
  throw new Error(
    `Tauri binary not found: ${appBinaryPath}. Run \"npm run e2e:build\" first.`,
  );
}

if (process.platform === 'linux') {
  const check = spawnSync('sh', ['-lc', 'command -v WebKitWebDriver'], {
    stdio: 'pipe',
  });
  if (check.status !== 0) {
    throw new Error(
      'WebKitWebDriver is required for Tauri WebDriver tests on Linux. Install package: webkit2gtk-driver',
    );
  }
}

const e2eHome = join(appPath, 'e2e', '.home', `${Date.now()}`);
if (!existsSync(e2eHome)) {
  mkdirSync(e2eHome, { recursive: true });
}

const appEnv: Record<string, string> = {
  HOME: e2eHome,
  PATH:
    process.platform === 'linux'
      ? process.env.PATH ||
        '/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin'
      : process.env.PATH || '',
};

for (const key of [
  'DISPLAY',
  'WAYLAND_DISPLAY',
  'XDG_RUNTIME_DIR',
  'DBUS_SESSION_BUS_ADDRESS',
  'LANG',
  'LC_ALL',
]) {
  const value = process.env[key];
  if (value) {
    appEnv[key] = value;
  }
}

type TauriCapability = {
  browserName: 'tauri';
  browserVersion?: string;
  platformName?: string;
  'tauri:options': {
    application: string;
    args?: string[];
  };
  'wdio:tauriServiceOptions': {
    appBinaryPath: string;
    appArgs: string[];
    env: Record<string, string>;
    driverProvider: 'official' | 'embedded';
    autoInstallTauriDriver: boolean;
    captureBackendLogs?: boolean;
    captureFrontendLogs?: boolean;
    backendLogLevel?: 'trace' | 'debug' | 'info' | 'warn' | 'error';
    frontendLogLevel?: 'trace' | 'debug' | 'info' | 'warn' | 'error';
  };
};

const capabilities: TauriCapability[] = [
  {
    browserName: 'tauri',
    browserVersion: 'tauri',
    platformName: process.platform,
    'tauri:options': {
      application: appBinaryPath,
    },
    'wdio:tauriServiceOptions': {
      appBinaryPath,
      appArgs: [],
      env: appEnv,
      driverProvider: 'embedded',
      autoInstallTauriDriver: true,
      captureBackendLogs: true,
      captureFrontendLogs: true,
      backendLogLevel: 'info',
      frontendLogLevel: 'info',
    },
  },
];

export const config = {
  runner: 'local',
  specs: ['./e2e/specs/**/*.spec.ts'],
  exclude: [],
  maxInstances: 1,
  capabilities,
  logLevel:
    process.env.WDIO_LOG_LEVEL || (process.env.DEBUG ? 'debug' : 'info'),
  groupLogsByTestSpec: false,
  outputDir: './e2e/logs',
  bail: 0,
  baseUrl: '',
  waitforTimeout: 15000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  autoXvfb: false,
  services: [
    [
      '@wdio/tauri-service',
      {
        driverProvider: 'embedded',
        autoInstallTauriDriver: true,
      },
    ],
  ],
  framework: 'mocha',
  reporters: [['spec', { stdout: true }]],
  beforeSession: (_config: unknown, caps: Record<string, unknown>) => {
    if (!caps.browserName) {
      caps.browserName = 'tauri';
    }
    if (!caps.browserVersion) {
      caps.browserVersion = 'tauri';
    }
    if (!caps.platformName) {
      caps.platformName = process.platform;
    }
  },
  mochaOpts: {
    ui: 'bdd',
    timeout: 120000,
  },
  tsConfigPath: join(__dirname, 'e2e', 'tsconfig.json'),
};
