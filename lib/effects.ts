import React from 'react';
import { Subject } from 'rxjs';
import { debounceTime } from 'rxjs/operators';

import type { AppState } from '@/types/app';
import type { Profile, ProfilePartial } from '@/types/profile';
import { AppLoaderProps } from '@/components/app-loader';

export function useInvoke<T, I>(initialValue: I, cmd: string, args?: any) {
  const [data, setData] = React.useState<T | I>(initialValue);
  const [err, setErr] = React.useState<string | null>(null);
  const [isLoading, setisLoading] = React.useState<boolean>(true);
  const fetch = React.useCallback(() => {
    setisLoading(true);
    require('@tauri-apps/api/core')
      .invoke(cmd, args)
      .then((d: T) => setData(d))
      .catch((error: any) => setErr(error))
      .finally(() => setisLoading(false));
  }, [args, cmd]);
  React.useEffect(() => fetch(), [fetch]);
  return [data, err, isLoading, setData, fetch] as const;
}

export function useAppState() {
  return useInvoke<AppState, AppState>({}, 'get_state');
}

export function useProfiles() {
  return useInvoke<Profile[], Profile[]>([], 'list_profile');
}

export function disconnect(
  onfinally?: () => void,
  onerror?: (err: any) => void,
) {
  require('@tauri-apps/api/core')
    .invoke('disconnect')
    .catch(onerror)
    .finally(onfinally);
}

export function connect(
  profile: string,
  onfinally?: () => void,
  onerror?: (err: any) => void,
) {
  require('@tauri-apps/api/core')
    .invoke('connect_profile', { profile })
    .catch(onerror)
    .finally(onfinally);
}

export function useAppLoader(isOpen: boolean = false) {
  const [appLoader, setAppLoader] = React.useState<AppLoaderProps>({
    kind: 'Connecting',
    isOpen,
  });
  return [appLoader, setAppLoader] as const;
}

export function createProfile<T>(
  newProfile: ProfilePartial,
  onsuccess?: (d: T) => void,
  onerror?: (err: any) => void,
  onfinally?: () => void,
) {
  return require('@tauri-apps/api/core')
    .invoke('create_profile', { newProfile })
    .then(onsuccess)
    .catch(onerror)
    .finally(onfinally);
}

export function deleteProfile<T>(
  profileName: string,
  onsuccess?: (d: T) => void,
  onerror?: (err: any) => void,
  onfinally?: () => void,
) {
  return require('@tauri-apps/api/core')
    .invoke('delete_profile', { profileName })
    .then(onsuccess)
    .catch(onerror)
    .finally(onfinally);
}

export function updateProfile<T>(
  profileName: string,
  profile: ProfilePartial,
  onsuccess?: (d: T) => void,
  onerror?: (err: any) => void,
  onfinally?: () => void,
) {
  return require('@tauri-apps/api/core')
    .invoke('update_profile', { profileName, profile })
    .then(onsuccess)
    .catch(onerror)
    .finally(onfinally);
}

export const useDebounce = (time: number, initialValue: any) => {
  const [value, setValue] = React.useState(initialValue);
  const [values] = React.useState(() => new Subject());
  React.useEffect(() => {
    const sub = values.pipe(debounceTime(time)).subscribe(setValue);
    return () => sub.unsubscribe();
  }, [time, values]);
  const newSetValue = React.useCallback((v: any) => values.next(v), [values]);
  return [value, newSetValue] as const;
};
