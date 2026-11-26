# Repository conventions

- Include a Changesets entry in `.changeset/` for each well-defined change. This only pertains to code changes or changes to public facing documentation. Changesets are ONLY for changes to published artifacts e.g. packages. Don't include any for CI/docs/unrelated files
- Use Angular-style Conventional Commit syntax for pull request titles, with an optional scope and an imperative summary. Examples include `feat(compiler): support media queries` and `fix: preserve stylesheet source order`.
