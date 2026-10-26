import assert from 'node:assert/strict';
import test from 'node:test';

// @ts-expect-error Node runs this test directly using built-in TypeScript support.
import { orderProfiles } from '../lib/profile-ordering.ts';

const profiles = Object.freeze(
  ['delta', 'bravo', 'alpha', 'charlie'].map((name) =>
    Object.freeze({ name, content: '' }),
  ),
);
const names = (result: Readonly<ReturnType<typeof orderProfiles>>) => result.map((profile) => profile.name);

test('orders alphabetically without changing the source state array', () => {
  assert.deepEqual(names(orderProfiles(profiles)), ['alpha', 'bravo', 'charlie', 'delta']);
  assert.deepEqual(names(profiles), ['delta', 'bravo', 'alpha', 'charlie']);
});

test('pins the current profile and keeps all other profiles alphabetical', () => {
  assert.deepEqual(names(orderProfiles(profiles, 'charlie')), ['charlie', 'alpha', 'bravo', 'delta']);
  assert.deepEqual(names(orderProfiles(profiles, 'missing')), ['alpha', 'bravo', 'charlie', 'delta']);
});

test('filters case-insensitively while preserving alphabetical and current ordering', () => {
  assert.deepEqual(names(orderProfiles(profiles, 'delta', 'A')), ['delta', 'alpha', 'bravo', 'charlie']);
  assert.deepEqual(names(orderProfiles(profiles, 'delta', 'AL')), ['alpha']);
  assert.deepEqual(orderProfiles(profiles, 'delta', 'no-match'), []);
});


test('orders mixed-case names alphabetically', () => {
  const mixedCase = ['Zulu', 'alpha', 'Bravo', 'charlie'].map((name) => ({ name, content: '' }));
  assert.deepEqual(names(orderProfiles(mixedCase)), ['alpha', 'Bravo', 'charlie', 'Zulu']);
  assert.deepEqual(names(orderProfiles(mixedCase, 'charlie')), ['charlie', 'alpha', 'Bravo', 'Zulu']);
});
