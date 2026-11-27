---
"@csx-style/compiler": patch
---

Emit three CSX cascade layers for resets, shorthands, and longhands. Preserve media and selector-state precedence in a single emitted stylesheet through rule order and equal-specificity selectors. The `revert-layer` keyword now rolls back the entire property tier.
