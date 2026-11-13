// Dependency-free fixture: typed integer helpers, exported for the test runner.
// `tsc --noEmit` type-checks this file; `node --test` runs it with type
// stripping. Type stripping erases annotations, it does not check them: the
// compiler is the type gate, the test runner is the execution gate.

export function add(left: number, right: number): number {
  return left + right;
}

/** Sum of a list, starting from 0. Accepts a readonly view deliberately. */
export function sum(values: readonly number[]): number {
  return values.reduce((total, value) => total + value, 0);
}
