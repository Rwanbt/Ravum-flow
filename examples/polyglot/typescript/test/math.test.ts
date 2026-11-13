// Run with `node --test test/math.test.ts` from the repository root.
// Node >= 22.6 strips the type annotations before executing this file.
import test from 'node:test';
import assert from 'node:assert/strict';

import { add, sum } from '../src/math.ts';

test('adds positive numbers', () => {
  assert.equal(add(2, 3), 5);
});

test('adds negative numbers', () => {
  assert.equal(add(-2, 3), 1);
});

test('sums a list', () => {
  assert.equal(sum([1, 2, 3]), 6);
});
