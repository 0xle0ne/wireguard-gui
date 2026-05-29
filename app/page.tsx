'use client';

import { Suspense, useCallback, useEffect, useState } from 'react';
import Image from 'next/image';
import { getVersion } from '@tauri-apps/api/app';
import { Lock, PowerOff, Unlock } from 'lucide-react';
import { toast } from 'sonner';

import {
  CommandError,
  connect,
  disconnect,
  useAppLoader,
  useAppState,
} from '@/lib/effects';
import { Button } from '@/components/ui/button';
import { AppLoader } from '@/components/app-loader';
import { ProfileTable } from '@/components/profile-table';
import { AppSplashScreen } from '@/components/app-splash-screen';

export default function Index() {
  const [showSplash, setShowSplash] = useState(true);
  const [state, , , , fetchState] = useAppState();
  const [appLoader, setAppLoader] = useAppLoader();
  const [appVersion, setAppVersion] = useState<string | null>(null);

  useEffect(() => {
    getVersion()
      .then((v) => {
        setAppVersion(v);
      })
      .catch(() => {
        setAppVersion('unknown');
      })
      .finally(() => {
        setTimeout(() => setShowSplash(false), 1000);
      });
  }, []);

  const onConnectionFinish = useCallback(() => {
    return () => {
      fetchState();
      setAppLoader((l) => ({ ...l, isOpen: false }));
    };
  }, [fetchState, setAppLoader]);

  const onError = useCallback((commandError: CommandError) => {
    const byCode: Record<string, string> = {
      activation_failed:
        'Failed to activate connection. Check your profile and NetworkManager logs.',
      import_failed:
        'Profile import failed. Verify the WireGuard config content.',
      invalid_profile_name:
        'Invalid profile name. Use 1-15 characters: alphanumeric, _, -, ., =',
      nmcli_missing: 'NetworkManager CLI is missing. Snap mode requires nmcli.',
      permission_denied:
        'Permission denied. Check polkit/network-manager permissions.',
      profile_exists: 'A profile with this name already exists.',
      profile_not_found: 'Profile no longer exists on disk.',
      script_failed: 'Connection script failed. Check logs for details.',
      timeout: 'Network operation timed out. Please retry.',
    };

    const description =
      (commandError.code ? byCode[commandError.code] : undefined) ||
      commandError.message ||
      'Unknown error';

    toast.error('Connection error', { description });
  }, []);

  const onConnect = useCallback(
    (profile: string) => {
      return () => {
        setAppLoader({
          kind: 'Connecting',
          isOpen: true,
          message: `Connecting to ${profile}`,
        });
        connect(profile, onConnectionFinish(), onError);
      };
    },
    [setAppLoader, onConnectionFinish, onError],
  );

  // eslint-disable-next-line react-hooks/preserve-manual-memoization
  const onDisconnect = useCallback(() => {
    setAppLoader({
      kind: 'Disconnecting',
      isOpen: true,
      message: `Disconnecting from ${state.current}`,
    });
    disconnect(onConnectionFinish(), onError);
  }, [state, setAppLoader, onConnectionFinish, onError]);

  return (
    <div className="bg-background h-screen">
      {showSplash && <AppSplashScreen />}
      <AppLoader {...appLoader} />
      <div className="m-auto flex max-w-(--breakpoint-lg) flex-col px-4 pt-4">
        <div className="mb-8 flex items-center justify-between">
          <Image
            title="Wireguard GUI"
            alt="wireguard"
            src="/img/wireguard.png"
            width={42}
            height={42}
          />
          <strong>v{appVersion}</strong>
          <Button
            disabled={state?.conn_st !== 'Connected'}
            title="disconnect"
            variant={state?.conn_st === 'Connected' ? 'destructive' : null}
            className="ml-2"
            onClick={onDisconnect}
          >
            <PowerOff className="size-4" />
          </Button>
        </div>
        <div className="mb-8 flex flex-col items-center justify-center">
          {state.conn_st === 'Connected' ? (
            <Lock className="mb-2 size-16 text-green-500" />
          ) : (
            <Unlock className="animate-pulsemb-2 size-16 text-red-500" />
          )}
          <p className="mt-2 font-bold">{state.current || 'Not connected'}</p>
          <p className="font-bold">{state?.pub_ip || 'ip undetected'}</p>
        </div>
        <Suspense>
          <ProfileTable current={state?.current} onConnect={onConnect} />
        </Suspense>
      </div>
    </div>
  );
}
