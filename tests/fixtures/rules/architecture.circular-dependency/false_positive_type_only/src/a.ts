// A type-only import cycle: the TypeScript compiler erases `import type`, so
// `a` and `b` referencing each other's types creates no runtime dependency and
// must not be reported as a circular dependency.
import type { B } from "./b";

export type A = { value: number; other: B };
