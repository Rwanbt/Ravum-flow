// Hand-written minimal declarations for the two built-in modules this fixture
// imports. `@types/node` cannot be fetched (the fixture is offline and installs
// nothing), so the exact surface that is used is declared here. This is not a
// copy of the Node typings and makes no claim to be one.

declare module 'node:test' {
  type TestFn = () => void | Promise<void>;
  export default function test(name: string, fn: TestFn): void;
}

declare module 'node:assert/strict' {
  interface Assert {
    equal(actual: unknown, expected: unknown, message?: string): void;
  }
  const assert: Assert;
  export default assert;
}
