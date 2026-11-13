'use strict';

// Run with `node --test` from the repository root.
const test = require('node:test');
const assert = require('node:assert/strict');

const { add } = require('../src/math.js');

test('adds positive numbers', () => {
  assert.equal(add(2, 3), 5);
});

test('adds negative numbers', () => {
  assert.equal(add(-2, 3), 1);
});
