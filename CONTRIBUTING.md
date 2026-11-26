# Contributing to CSX

Thank you for taking an interest in contributing to CSX. Contributions of all sizes are welcome, whether they involve fixing a typo, improving diagnostics, resolving a compiler bug, or proposing a new language feature.

This document outlines how contributions are made, reviewed, and accepted into the project.

The [code of conduct](CODE_OF_CONDUCT.md) applies to all CSX contributors. See the [security policy](SECURITY.md) for how to report a vulnerability.

## 1. Project principles

CSX is developed with the following principles in mind:

- **CSS first.** CSX should remain familiar to developers who already understand CSS.
- **Predictability.** Developers should be able to reason about the behaviour and output of their styles.
- **Correctness.** Compiler correctness takes precedence over implementation convenience.
- **Interoperability.** CSX should work alongside the wider CSS and JavaScript ecosystems rather than unnecessarily replacing them.
- **Developer experience.** Complexity should be handled by the tooling wherever reasonably possible.
- **Maintainability.** Contributions should consider the long-term health of the project, not just the immediate problem being solved.

These principles should guide both implementation and language design decisions.

## 2. Ways to contribute

Contributions are not limited to writing code. You can contribute by:

- Reporting bugs and unexpected behaviour.
- Proposing language and tooling improvements.
- Improving documentation and examples.
- Improving compiler diagnostics.
- Adding test coverage.
- Investigating performance issues.
- Reviewing existing proposals and contributions.
- Developing and improving integrations.

You do not need to be familiar with every part of CSX to contribute.

When reporting a bug, include the CSX package versions, the build tool and its version, and the smallest `.x.css` file that reproduces it. Include the import, `props()` call, or PostCSS configuration if it affects the result. Show what you expected and what CSX produced, such as generated CSS, a diagnostic, or a browser rendering.

## 3. Before starting work

Small, self-contained improvements generally do not require prior approval.

However, please open or participate in an issue or discussion before beginning work on changes that:

- Introduce or modify language syntax.
- Change compiler semantics or output guarantees.
- Affect backwards compatibility.
- Significantly restructure existing implementations.
- Introduce substantial dependencies.
- Introduce new public APIs or configuration options.

This allows contributors and maintainers to agree on the intended behaviour before significant implementation work takes place.

Implementing something does not guarantee that its underlying proposal will be accepted.

## 4. Development environment

### Prerequisites

Use the toolchain in [CI](.github/workflows/ci.yml) when reproducing a build or test failure.

- Node.js 24.
- Bun. The workspace version is recorded in [package.json](package.json).
- Stable Rust through rustup, with the `wasm32-unknown-unknown` target.
- wasm-pack 0.15.1 to build the Rust compiler for JavaScript consumers.

CI runs on Ubuntu. The commands below assume a POSIX shell.

### Getting started

Fork CSX, clone your fork, and create a branch from `main`. Run these commands from the clone:

```sh
rustup toolchain install stable --target wasm32-unknown-unknown
rustup override set stable
cargo install wasm-pack --version 0.15.1 --locked
bun install --frozen-lockfile
```

The workspace links the examples to the local CSX packages. You can test your changes without publishing packages.

### Building CSX

Run `bun run build` from the repository root to build the compiler, integrations, and example apps. The compiler package builds the Rust crate to WebAssembly and bundles it for Node.js and browsers.

For a focused build, use a Turborepo filter. For example, build the Vite integration and its workspace dependencies with `bun run build --filter=@csx-style/vite`. Use `bun run build:rust` for a native Rust build.

After building the workspace, run `bun --cwd examples/vite-react run dev` or `bun --cwd examples/next run dev` to try changes in an app. Rebuild the affected packages after editing their source so the app picks up the updated output.

### Running tests

From the repository root, run `bun run format:check`, `cargo fmt --all --check`, `bun run build`, and `bun run test` to match CI. Run `bun run check` to also check TypeScript types and run Clippy.

For a narrower check, use `bun run test:rust` for the Rust workspace or `bun run test --filter=@csx-style/compiler` for the compiler package's JavaScript and WASM tests. The same filter accepts integration and example package names, such as `@csx-style/postcss` and `@csx-style/example-next`.

### Repository structure

| Path                                                              | Start here for                                                                                 |
| ----------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| [crates/compiler](crates/compiler)                                | Parsing `.x.css`, generating atomic CSS, and ordering the emitted cascade                      |
| [packages/compiler](packages/compiler)                            | The WebAssembly-backed compiler package, JavaScript module output, and TypeScript declarations |
| [packages/csx](packages/csx)                                      | The `@csx-style/core` runtime, including `props()` and style composition                       |
| [packages/vite](packages/vite) and [packages/next](packages/next) | Loading `.x.css` imports in Vite and Next.js                                                   |
| [packages/postcss](packages/postcss)                              | Expanding `@csx;` into generated CSS                                                           |
| [examples](examples)                                              | Vite and Next.js apps for trying local changes end to end                                      |
| [README.md](README.md) and the package READMEs                    | User-facing syntax, setup, and integration guidance                                            |

## 5. Engineering guidelines

### Scope

Keep contributions focused on the problem they intend to solve.

Avoid introducing unrelated refactors, formatting changes, dependency updates, or behavioural changes within the same pull request unless they are necessary for its implementation.

### Implementation

Contributions should:

- Follow existing architectural and coding conventions.
- Prefer straightforward implementations over unnecessary abstractions.
- Handle errors explicitly and provide useful diagnostics.
- Avoid introducing undocumented assumptions.
- Consider the performance implications of compiler changes.
- Preserve existing behaviour unless a change has been explicitly agreed upon.

Existing code should not be treated as inherently correct. Improvements to established implementations are welcome when supported by a clear justification.

### Dependencies

New dependencies should have a clear purpose and provide sufficient benefit to justify their inclusion.

Consider their maintenance status, licensing, portability, security, and impact on build times and distribution size.

### Compiler behaviour

Changes to compiler behaviour should consider:

- Deterministic compilation.
- CSS specification compatibility.
- Style composition and precedence.
- Token resolution and theming.
- Cross-file and cross-module behaviour.
- Generated CSS correctness.
- Error handling and diagnostics.
- Compatibility with supported build tools.

Where applicable, contributions should include representative input and expected output examples.

For changes to precedence, use styles that collide on the same property. Verify both the compiler's CSS order and the result of composing the relevant styles with `props()`. Include overlapping selectors or media queries when they are part of the change.

## 6. Testing and verification

All implementation changes must be appropriately tested.

Bug fixes should generally include a regression test that fails before the fix and passes afterwards.

New features should include tests covering their expected behaviour, relevant edge cases, and invalid inputs.

Where appropriate, consider:

- Unit tests.
- Integration tests.
- Compiler input/output tests.
- Regression tests.
- Diagnostic tests.
- Performance benchmarks.

Passing automated tests does not remove the responsibility to manually review a contribution.

If a change cannot reasonably be covered by automated tests, explain why and provide an alternative method of verification in the pull request.

Any known test failures or limitations must be disclosed.

- For compiler output or diagnostics, start with the [compiler tests](packages/compiler/tests). Include a minimal `.x.css` input and assert the CSS, exported styles, declarations, or diagnostic affected by the change. Review snapshot diffs before accepting them.
- For style composition, extend the [core tests](packages/csx/tests). For `@csx;` expansion, use the [PostCSS tests](packages/postcss/tests).
- For changes that cross an integration boundary, use the [Vite HMR test](examples/vite-js/tests/hmr.test.ts) or the [Next.js build test](examples/next/tests/build.test.ts) as a starting point.

## 7. Documentation

Changes to public-facing behaviour should be accompanied by corresponding documentation updates.

This includes changes to:

- Language syntax and semantics.
- Configuration.
- Public APIs.
- Compiler diagnostics.
- Build tool integrations.
- Supported environments.
- Existing documented guarantees.

Documentation should describe behaviour from the perspective of the developer using CSX, rather than solely describing its internal implementation.

Breaking changes should include migration guidance where applicable.

Update the root [README](README.md) for changes to CSX syntax or shared setup. Put integration-specific instructions in the relevant package README and update an example when it demonstrates the affected behaviour.

## 8. AI-assisted contributions

Read the [AI contribution policy](AI_POLICY.md) for the author and peer review requirements. You remain responsible for the quality, correctness, and licensing of everything you submit.

Confirm your manual review in the pull request checklist. When your pull request is ready for review, GitHub Actions leaves a reminder if **What**, **Why**, or **How** is empty or if an item in the **Contributor checklist** is unchecked. Updating the description refreshes the reminder, and completing it removes the comment.

## 9. Submitting a pull request

Before opening a pull request:

1. Ensure your changes are appropriately scoped.
2. Review your complete diff.
3. Run the relevant tests and checks.
4. Update documentation where necessary.
5. Add a changeset for each well-defined code or public-facing documentation change.
6. Complete the [pull request template](.github/pull_request_template.md).
7. Disclose known limitations, failures, and breaking changes.

Pull requests should explain what changed, why it changed, and how the implementation addresses the problem.

Use an Angular-style Conventional Commit title with an imperative summary. The scope is optional, as in `feat(compiler): support media queries` or `fix: preserve stylesheet source order`. The PR template check fails on invalid title formatting and reruns when you edit the title.

Draft pull requests are welcome when seeking early feedback on an incomplete implementation.

### Changesets

Run `bun run changeset` from the repository root to add an entry in [`.changeset/`](.changeset). Select the affected published packages and the appropriate version bump. Describe the change in terms users will see in the changelog, including migration steps when relevant. The published packages are versioned together by the release workflow.
