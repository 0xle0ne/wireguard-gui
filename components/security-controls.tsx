'use client';

import React from 'react';
import { KeyRound, RotateCcw, Shield, ShieldOff } from 'lucide-react';
import { toast } from 'sonner';

import type { CommandError } from '@/lib/effects';
import {
  disableProfileEncryption,
  enableProfileEncryption,
  lockProfiles,
  resetAppData,
  unlockProfiles,
} from '@/lib/effects';
import { AlertConfirm } from '@/components/ui/alert-confirm';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';

function mapSecurityError(error: CommandError) {
  const byCode: Record<string, string> = {
    encryption_already_enabled: 'Encryption is already enabled.',
    encryption_not_enabled: 'Encryption is not enabled.',
    invalid_pin: 'PIN must be exactly 4 digits.',
    pin_incorrect: 'Incorrect PIN.',
    profiles_locked: 'Profiles are locked. Unlock first.',
  };
  return byCode[error.code || ''] || error.message || 'Unknown error';
}

function isPinFormatValid(pin: string) {
  return /^\d{4}$/.test(pin);
}

export interface SecurityControlsProps {
  encryptionEnabled?: boolean;
  isUnlocked?: boolean;
  onStateChanged: () => void;
}

export function SecurityControls({
  encryptionEnabled,
  isUnlocked,
  onStateChanged,
}: SecurityControlsProps) {
  const [enablePin, setEnablePin] = React.useState('');
  const [disablePin, setDisablePin] = React.useState('');
  const [confirmResetOpen, setConfirmResetOpen] = React.useState(false);

  const onEnable = React.useCallback(() => {
    if (!isPinFormatValid(enablePin)) {
      toast.error('Invalid PIN', {
        description: 'PIN must be exactly 4 digits.',
      });
      return;
    }
    enableProfileEncryption(
      enablePin,
      () => {
        setEnablePin('');
        toast.success('Profile encryption enabled');
        onStateChanged();
      },
      (error) =>
        toast.error('Failed to enable encryption', {
          description: mapSecurityError(error),
        }),
    );
  }, [enablePin, onStateChanged]);

  const onDisable = React.useCallback(() => {
    if (!isPinFormatValid(disablePin)) {
      toast.error('Invalid PIN', {
        description: 'PIN must be exactly 4 digits.',
      });
      return;
    }
    disableProfileEncryption(
      disablePin,
      () => {
        setDisablePin('');
        toast.success('Profile encryption disabled');
        onStateChanged();
      },
      (error) =>
        toast.error('Failed to disable encryption', {
          description: mapSecurityError(error),
        }),
    );
  }, [disablePin, onStateChanged]);

  const onLockNow = React.useCallback(() => {
    lockProfiles(
      () => {
        toast.success('Profiles locked');
        onStateChanged();
      },
      (error) =>
        toast.error('Failed to lock', { description: mapSecurityError(error) }),
    );
  }, [onStateChanged]);

  const onConfirmReset = React.useCallback(() => {
    resetAppData(
      () => {
        setConfirmResetOpen(false);
        setEnablePin('');
        setDisablePin('');
        toast.success('App data reset complete');
        onStateChanged();
      },
      (error) => {
        toast.error('Failed to reset app', {
          description: mapSecurityError(error),
        });
      },
    );
  }, [onStateChanged]);

  return (
    <>
      <AlertConfirm
        isOpen={confirmResetOpen}
        setOpen={setConfirmResetOpen}
        onConfirm={onConfirmReset}
        title="Reset app data?"
        description="This will permanently remove all local profiles and security settings. This cannot be undone."
      />
      <Dialog>
        <DialogTrigger asChild>
          <Button
            title="Security settings"
            variant="outline"
            size="icon"
            data-testid="security-open"
          >
            <Shield className="size-4" />
          </Button>
        </DialogTrigger>
        <DialogContent aria-describedby="security-dialog">
          <DialogHeader>
            <DialogTitle>Security</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            {!encryptionEnabled ? (
              <form
                className="space-y-2"
                data-testid="security-enable-section"
                onSubmit={(event) => {
                  event.preventDefault();
                  onEnable();
                }}
              >
                <p className="text-sm text-muted-foreground">
                  Enable at-rest encryption for local WireGuard profiles using a
                  4-digit PIN.
                </p>
                <Input
                  type="password"
                  inputMode="numeric"
                  maxLength={4}
                  placeholder="4-digit PIN"
                  value={enablePin}
                  onChange={(e) =>
                    setEnablePin(e.target.value.replace(/\D/g, ''))
                  }
                  data-testid="security-enable-pin"
                />
                <Button
                  type="submit"
                  className="w-full"
                  data-testid="security-enable-submit"
                >
                  <KeyRound className="size-4" />
                  Enable encryption
                </Button>
              </form>
            ) : (
              <form
                className="space-y-2"
                data-testid="security-disable-section"
                onSubmit={(event) => {
                  event.preventDefault();
                  onDisable();
                }}
              >
                <p className="text-sm text-muted-foreground">
                  Encryption is active. If you forget the PIN, use full app
                  reset.
                </p>
                <Input
                  type="password"
                  inputMode="numeric"
                  maxLength={4}
                  placeholder="Current PIN"
                  value={disablePin}
                  onChange={(e) =>
                    setDisablePin(e.target.value.replace(/\D/g, ''))
                  }
                  data-testid="security-disable-pin"
                />
                <Button
                  type="submit"
                  className="w-full"
                  data-testid="security-disable-submit"
                >
                  <ShieldOff className="size-4" />
                  Disable encryption
                </Button>
                {isUnlocked ? (
                  <Button
                    type="button"
                    variant="outline"
                    className="w-full"
                    onClick={onLockNow}
                    data-testid="security-lock-now"
                  >
                    Lock now
                  </Button>
                ) : null}
              </form>
            )}

            <Button
              type="button"
              variant="destructive"
              className="w-full"
              onClick={() => setConfirmResetOpen(true)}
              data-testid="security-reset"
            >
              <RotateCcw className="size-4" />
              Reset app data
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

export interface UnlockPanelProps {
  onUnlocked: () => void;
  onReset: () => void;
}

export function UnlockPanel({ onUnlocked, onReset }: UnlockPanelProps) {
  const [pin, setPin] = React.useState('');

  const onUnlock = React.useCallback(() => {
    if (!isPinFormatValid(pin)) {
      toast.error('Invalid PIN', {
        description: 'PIN must be exactly 4 digits.',
      });
      return;
    }
    unlockProfiles(
      pin,
      () => {
        setPin('');
        toast.success('Profiles unlocked');
        onUnlocked();
      },
      (error) =>
        toast.error('Unlock failed', { description: mapSecurityError(error) }),
    );
  }, [pin, onUnlocked]);

  return (
    <div className="rounded-lg border p-6" data-testid="unlock-panel">
      <h2 className="mb-2 text-lg font-semibold">Profiles are locked</h2>
      <p className="mb-4 text-sm text-muted-foreground">
        Enter your 4-digit PIN to unlock encrypted profiles.
      </p>
      <form
        className="flex gap-2"
        onSubmit={(event) => {
          event.preventDefault();
          onUnlock();
        }}
      >
        <Input
          type="password"
          inputMode="numeric"
          maxLength={4}
          placeholder="4-digit PIN"
          value={pin}
          onChange={(e) => setPin(e.target.value.replace(/\D/g, ''))}
          data-testid="unlock-pin"
        />
        <Button type="submit" data-testid="unlock-submit">
          Unlock
        </Button>
      </form>
      <Button
        type="button"
        variant="destructive"
        className="mt-4 w-full"
        onClick={onReset}
        data-testid="unlock-reset"
      >
        Reset app data
      </Button>
    </div>
  );
}
