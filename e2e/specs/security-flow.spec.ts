import { $, browser, expect } from '@wdio/globals';

describe('Security flow', () => {
  const profileName = `e2e-${Date.now().toString().slice(-6)}`;

  const profileContent = `[Interface]
Address = 10.10.10.2/24
PrivateKey = fakeprivatekey

[Peer]
PublicKey = fakepubkey
AllowedIPs = 0.0.0.0/0
Endpoint = 127.0.0.1:51820`;

  const updatedProfileContent = `${profileContent}\n# updated-by-e2e`;

  async function tauriInvoke<T = unknown>(cmd: string, args?: Record<string, unknown>) {
    return browser.execute(
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      async (command: string, payload: Record<string, unknown>) => {
        const tauri = (window as any).__TAURI_INTERNALS__;
        if (!tauri?.invoke) {
          throw new Error('Tauri invoke bridge is unavailable in the test webview');
        }
        return tauri.invoke(command, payload);
      },
      cmd,
      args || {},
    ) as Promise<T>;
  }

  async function step<T>(
    name: string,
    action: () => Promise<T>,
    timeout = 30000,
  ) {
    console.log(`[e2e] start: ${name}`);
    const timer = new Promise<never>((_, reject) => {
      setTimeout(
        () => reject(new Error(`Step timeout after ${timeout}ms: ${name}`)),
        timeout,
      );
    });

    const result = await Promise.race([action(), timer]);
    console.log(`[e2e] done: ${name}`);
    return result;
  }

  async function waitForHidden(selector: string, timeout = 20000) {
    await browser.waitUntil(
      async () => {
        const element = await $(selector);
        if (!(await element.isExisting())) {
          return true;
        }
        return !(await element.isDisplayed());
      },
      {
        timeout,
        timeoutMsg: `Element still visible after ${timeout}ms: ${selector}`,
      },
    );
  }

  async function waitForAppReady() {
    await waitForHidden('[data-testid="app-splash"]', 30000);
    const securityButton = await $('[data-testid="security-open"]');
    await securityButton.waitForDisplayed({ timeout: 30000 });
  }

  async function setKeepAlive(enabled: boolean) {
    await browser.execute((active: boolean) => {
      const w = window as unknown as {
        __e2eKeepAliveInterval?: number;
      };

      if (w.__e2eKeepAliveInterval) {
        window.clearInterval(w.__e2eKeepAliveInterval);
        w.__e2eKeepAliveInterval = undefined;
      }

      if (active) {
        w.__e2eKeepAliveInterval = window.setInterval(() => {
          window.dispatchEvent(new Event('mousemove'));
          window.dispatchEvent(new Event('keydown'));
          window.dispatchEvent(new Event('focus'));
        }, 2000);
      }
    }, enabled);
  }

  function getErrorText(error: unknown) {
    if (!error) {
      return '';
    }
    if (typeof error === 'string') {
      return error;
    }
    if (typeof error === 'object') {
      const e = error as { message?: string; code?: string };
      return `${e.code || ''} ${e.message || ''}`.trim();
    }
    return String(error);
  }

  it('enables encryption, locks, unlocks, and resets app data', async () => {
    try {
      await step('wait for app ready', async () => {
        await waitForAppReady();
        await expect(await $('[data-testid="security-open"]')).toBeDisplayed();
      }, 60000);

      await step('start keepalive', async () => {
        await setKeepAlive(true);
      });

      await step('create profile', async () => {
        await tauriInvoke('create_profile', {
          newProfile: {
            name: profileName,
            content: profileContent,
          },
        });

        await browser.waitUntil(
          async () => {
            const profiles = await tauriInvoke<Array<{ name: string }>>('list_profile');
            return profiles.some((p) => p.name === profileName);
          },
          {
            timeout: 20000,
            interval: 500,
            timeoutMsg: `Profile was not created: ${profileName}`,
          },
        );
      });

      await step('enable encryption', async () => {
        await tauriInvoke('enable_profile_encryption', { pin: '1234' });
        const state = await tauriInvoke<{
          encryption_enabled?: boolean;
          is_unlocked?: boolean;
        }>('get_state');
        await expect(Boolean(state.encryption_enabled)).toBe(true);
        await expect(Boolean(state.is_unlocked)).toBe(true);
      });

      await step('lock profiles', async () => {
        await tauriInvoke('lock_profiles');
        const unlockPanel = await $('[data-testid="unlock-panel"]');
        await unlockPanel.waitForDisplayed({ timeout: 20000 });
        await expect(unlockPanel).toBeDisplayed();
      });

      await step('unlock profiles', async () => {
        await tauriInvoke('unlock_profiles', { pin: '1234' });
        await browser.waitUntil(
          async () => {
            const state = await tauriInvoke<{
              encryption_enabled?: boolean;
              is_unlocked?: boolean;
            }>('get_state');
            return Boolean(state.encryption_enabled) && Boolean(state.is_unlocked);
          },
          {
            timeout: 30000,
            interval: 500,
            timeoutMsg: 'Backend state did not become unlocked after unlock_profiles',
          },
        );

        await browser.execute(() => {
          window.dispatchEvent(new Event('focus'));
        });
        await expect(await $('[data-testid="security-open"]')).toBeDisplayed();

        // Ensure profile is still visible after unlock.
        await browser.waitUntil(
          async () => {
            const profileCell = await $(`p=${profileName}`);
            return profileCell.isExisting() && profileCell.isDisplayed();
          },
          {
            timeout: 20000,
            interval: 500,
            timeoutMsg: `Profile not visible after unlock: ${profileName}`,
          },
        );
      }, 90000);

      await step('edit profile', async () => {
        await tauriInvoke('update_profile', {
          profileName,
          profile: {
            name: profileName,
            content: updatedProfileContent,
          },
        });

        await browser.waitUntil(
          async () => {
            const profiles = await tauriInvoke<Array<{ name: string; content: string }>>('list_profile');
            return profiles.some(
              (p) => p.name === profileName && p.content.includes('# updated-by-e2e'),
            );
          },
          {
            timeout: 20000,
            interval: 500,
            timeoutMsg: `Profile content was not updated: ${profileName}`,
          },
        );
      });

      await step('delete profile', async () => {
        await tauriInvoke('delete_profile', { profileName });
        await browser.waitUntil(
          async () => {
            const profiles = await tauriInvoke<Array<{ name: string }>>('list_profile');
            return !profiles.some((p) => p.name === profileName);
          },
          {
            timeout: 20000,
            interval: 500,
            timeoutMsg: `Profile was not deleted: ${profileName}`,
          },
        );
      });

      await step('lock again', async () => {
        await tauriInvoke('lock_profiles');
        const unlockPanel = await $('[data-testid="unlock-panel"]');
        await unlockPanel.waitForDisplayed({ timeout: 20000 });
        await expect(unlockPanel).toBeDisplayed();
      });

      await step('reset app from lock panel', async () => {
        await tauriInvoke('reset_app_data');
        await browser.waitUntil(
          async () => {
            const state = await tauriInvoke<{
              encryption_enabled?: boolean;
              is_unlocked?: boolean;
            }>('get_state');
            return !Boolean(state.encryption_enabled) && Boolean(state.is_unlocked);
          },
          {
            timeout: 30000,
            interval: 500,
            timeoutMsg: 'Backend state did not reset after reset_app_data',
          },
        );
      });

      await step('verify encryption disabled after reset', async () => {
        const state = await tauriInvoke<{
          encryption_enabled?: boolean;
          is_unlocked?: boolean;
        }>('get_state');
        await expect(Boolean(state.encryption_enabled)).toBe(false);
        await expect(Boolean(state.is_unlocked)).toBe(true);
      });
    } finally {
      await setKeepAlive(false);
    }
  });

  it('auto-locks after inactivity and keeps profiles inaccessible until unlock', async () => {
    const inactivityProfile = `idle-${Date.now().toString().slice(-6)}`;

    try {
      await step('wait for app ready', async () => {
        await waitForAppReady();
      }, 60000);

      await step('prepare clean state', async () => {
        await tauriInvoke('reset_app_data');
        await setKeepAlive(false);
      });

      await step('create encrypted candidate profile', async () => {
        await tauriInvoke('create_profile', {
          newProfile: {
            name: inactivityProfile,
            content: profileContent,
          },
        });
      });

      await step('enable encryption', async () => {
        await tauriInvoke('enable_profile_encryption', { pin: '1234' });
        const state = await tauriInvoke<{
          encryption_enabled?: boolean;
          is_unlocked?: boolean;
        }>('get_state');
        await expect(Boolean(state.encryption_enabled)).toBe(true);
        await expect(Boolean(state.is_unlocked)).toBe(true);
      });

      await step('wait for inactivity auto-lock', async () => {
        await browser.waitUntil(
          async () => {
            const state = await tauriInvoke<{
              encryption_enabled?: boolean;
              is_unlocked?: boolean;
            }>('get_state');
            return Boolean(state.encryption_enabled) && !Boolean(state.is_unlocked);
          },
          {
            timeout: 25000,
            interval: 500,
            timeoutMsg: 'Expected inactivity auto-lock did not occur within timeout',
          },
        );
      }, 30000);

      await step('verify locked state blocks profile read', async () => {
        let lockedError: unknown;
        try {
          await tauriInvoke('list_profile');
        } catch (error) {
          lockedError = error;
        }

        await expect(Boolean(lockedError)).toBe(true);
        const errorText = getErrorText(lockedError).toLowerCase();
        await expect(errorText.includes('profiles_locked') || errorText.includes('locked')).toBe(true);
      });
    } finally {
      await setKeepAlive(false);
      await tauriInvoke('reset_app_data');
    }
  });
});
