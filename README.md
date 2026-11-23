# CSX

CSX is a styling library like css-modules for writing CSS in a more traditional
style, but with the benefits of atomic css and compiler-driven precedence similar to StyleX. It supports
design systems through variables and themes. See an example of what it csx might output in [the vite example](./examples/vite-react/example-output.css)

## Quick start

In a Vite app, add the CSX plugin to your vite config.

```js
import csx from "@csx-style/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [csx()] });
```

Install `@csx-style/postcss` and `postcss`, then add the CSX plugin to your PostCSS config. Vite and Next.js use the same PostCSS setup.

```js
// postcss.config.mjs
export default {
    plugins: {
        "@csx-style/postcss": {},
    },
};
```

Put `@csx;` in a regular stylesheet, such as `styles.css`, and import that file
in your app entry point. The PostCSS plugin replaces the directive with the generated CSS.

For TypeScript, enable `allowArbitraryExtensions` and add `.csx/types` to `rootDirs` alongside your project source root. The Vite plugin generates mapped declarations when a component imports a CSX stylesheet.

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

The [Next.js App Router example](./examples/next) uses the [Next.js adapter](./packages/next) and the PostCSS plugin. Build and run it from the repo root.

```sh
bun run build --filter=@csx-style/example-next
bun --filter @csx-style/example-next start
```
