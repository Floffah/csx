# @csx-style/compiler

## 0.1.0

### Minor Changes

- Support `:active`, `:disabled`, `:focus-within`, `:required`, and `:invalid` on class selectors in CSX stylesheets, including inside media queries. ([#10](https://github.com/Floffah/csx/pull/10))

- Support `:focus` and `:focus-visible` on class selectors in CSX stylesheets, including inside media queries. ([#7](https://github.com/Floffah/csx/pull/7))

- Support top-level media queries in CSX stylesheets, including query lists and hover styles. Document the supported stylesheet features in the README. ([#4](https://github.com/Floffah/csx/pull/4))

### Patch Changes

- Emit three CSX cascade layers for resets, shorthands, and longhands. Preserve media and selector-state precedence in a single emitted stylesheet through rule order and equal-specificity selectors. The `revert-layer` keyword now rolls back the entire property tier. ([#25](https://github.com/Floffah/csx/pull/25))

- Use a single ordered list to define supported selector states and their cascade layers. ([#9](https://github.com/Floffah/csx/pull/9))

## 0.0.1

### Patch Changes

- 4bb73de: Remove `@tokens` support
- e1b3a49: Changes the class name format from c7 to c8x
- 4bb73de: Compiler throws errors when it encounters an unsupported css feature
- a03b03f: Properly documented & licensed
- cb6fec2: Support removing modules from the compiler state
- 7785495: Initial alpha release of the CSX compiler, runtime, and Vite plugin.

## 0.0.1-alpha.2

### Patch Changes

- a03b03f: Properly documented & licensed
- cb6fec2: Support removing modules from the compiler state

## 0.0.1-alpha.1

### Patch Changes

- 7785495: Initial alpha release of the CSX compiler, runtime, and Vite plugin.
