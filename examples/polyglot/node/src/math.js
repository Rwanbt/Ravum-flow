'use strict';

// Dependency-free fixture: one integer addition, exported for the test runner.
// No package manager and no dependency is involved.
function add(left, right) {
  return left + right;
}

module.exports = { add };
