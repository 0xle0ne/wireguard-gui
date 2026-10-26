import type { Profile } from '../types/profile';

export function orderProfiles(
  profiles: readonly Profile[],
  current?: string | null,
  filter = '',
): Profile[] {
  const query = filter.toLowerCase();
  return profiles
    .filter((profile) => profile.name.toLowerCase().includes(query))
    .sort((a, b) => {
      if (a.name === current && b.name !== current) return -1;
      if (b.name === current && a.name !== current) return 1;
      return a.name.localeCompare(b.name, undefined, { sensitivity: 'base' });
    });
}
