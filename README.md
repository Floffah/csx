# CSX

CSX is a styling library like css-modules for writing CSS in a more traditional
style, but with the benefits of atomic css and compiler-driven precedence similar to StyleX. It supports
design systems through variables and themes. See an example of what it csx might output in [the vite example](./examples/vite-react/example-output.css)

## Quick start

In a Vite app, add the CSX plugin to your vite c onfig.

```js
import csx from "@csx-style/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [csx()] });
```

Put `@csx;` in a regular stylesheet, such as `styles.css`, and import that file
in your app entry point. Vite replaces the directive with the generated CSS.

Write styles in `Button.x.css`.

```css
.button {
    padding: 0.5rem 1rem;
    background-color: #e2e8f0;
}

.primary {
    background-color: #bfdbfe;
}
```

Import the styles and spread the result of `props()` onto an element.

```jsx
import { props } from "@csx-style/core";
import React from "react";

import styles from "./Button.x.css";

export function Button({ primary }) {
    return (
        <button
            type="button"
            {...props(styles.button, primary && styles.primary)}
        >
            Save
        </button>
    );
}
```

When two styles set the same property, the later argument to `props()` wins.
Here, `primary` changes the background and keeps the padding from `button`.

## Run the example

With Bun, Rust, and `wasm-pack` installed, run these commands from the repo root.

```sh
bun install
bun run build --filter=@csx-style/example-vite-react
bun --filter @csx-style/example-vite-react dev
```

See the [React example](./examples/vite-react) for the full setup, including
[TypeScript configuration](./examples/vite-react/tsconfig.json).

## Releases

Run `bun run changeset` for changes that should ship. Select the affected
packages and choose `major` for breaking changes, `minor` for new features, or
`patch` for a fix you intend to release. Commit the generated Markdown file
with your change. Work that needs no release can skip the changeset.

Changesets keeps a release PR open on `main` with version updates, the Bun
lockfile, and package changelogs. The compiler, core, and Vite plugin share one
version and release together. Merging the release PR publishes to npm through
GitHub Actions using OIDC.

### First alpha

1. In GitHub under **Settings → Actions → General**, enable **Allow GitHub
   Actions to create and approve pull requests**.
2. Check out the generated release PR for `0.0.1-alpha.1`. Build the packages
   and publish each one interactively with an npm account that can publish to
   `@csx-style`. npm requires an existing package before you can configure
   trusted publishing.

    ```sh
    bun install --frozen-lockfile
    bun run build --filter='./packages/*'
    npm login
    npm publish ./packages/compiler --access public --tag alpha
    npm publish ./packages/csx --access public --tag alpha
    npm publish ./packages/vite --access public --tag alpha
    ```

3. Merge the first release PR after publishing all three packages. Before
   merging the next release PR, add a GitHub Actions trusted publisher in each
   package's npm settings. Use owner `Floffah`, repository `csx`, and workflow
   `release.yml`. Leave the environment blank and allow `npm publish`. The
   first OIDC publish must succeed within two days of creating the trust.
   [npm Trusted Publishing setup](https://docs.npmjs.com/trusted-publishers/)

### Alpha, release candidates, and stable

- The workspace starts in `alpha` mode. When `0.0.1` is ready, run
  `bun run changeset pre exit` and merge the change to `.changeset/pre.json`.
  The release PR promotes the packages to stable on npm's `latest` tag.
- To preview a major release from stable, run `bun run changeset pre enter rc`
  and commit `.changeset/pre.json` with a major changeset. If the major is
  already in alpha, change `tag` to `rc` in `.changeset/pre.json` and add a
  changeset for the candidate. npm retains the previous `alpha` tag while
  candidates go to `rc`. Run `bun run changeset pre exit` when the major is
  ready for `latest`.
