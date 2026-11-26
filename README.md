# CSX

CSX is a styling library like css-modules for writing CSS in a more traditional
style, but with the benefits of atomic css and compiler-driven precedence similar to StyleX. It supports
design systems through variables and themes. See an example of what it csx might output in [the vite example](./examples/vite-react/example-output.css).

Write styles in CSS and use them from JavaScript or TypeScript. When you combine styles with `props()`, the order you pass them in determines which style wins for each property.

## 1. Set up your build tool

Choose the instructions for your app.

### Next.js

Use Next.js 16 with the App Router and the default Turbopack bundler. Use Node.js 20.19 or later on the 20.x line, or Node.js 22.12 or later.

```sh
npm install @csx-style/core @csx-style/next
```

Wrap your existing Next.js configuration with `withCsx(nextConfig)`. If you have no options to pass, use `withCsx()`.

```js
// next.config.mjs
import withCsx from "@csx-style/next";

export default withCsx();
```

### Vite

Use Vite 8.3.7 or later in the 8.x line.

```sh
npm install @csx-style/core
npm install -D @csx-style/vite
```

Add `csx()` to the plugins in your Vite config.

```js
// vite.config.mjs
import csx from "@csx-style/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [csx()] });
```

## 2. Add the PostCSS plugin

Next.js and Vite use the same PostCSS setup.

```sh
npm install -D @csx-style/postcss postcss
```

Add the plugin to your existing PostCSS config or create one in your app directory.

```js
// postcss.config.mjs
export default {
    plugins: {
        "@csx-style/postcss": {},
    },
};
```

## 3. Load the CSX styles

Add `@csx;` to a regular stylesheet, such as `globals.css`. This is where the generated CSS appears.

```css
/* globals.css */
@csx;
```

Import the stylesheet once from your Next.js root layout or Vite app entry point. If your app already imports a global stylesheet there, add the directive to that file.

```js
import "./globals.css";
```

## 4. Write and use a CSX style

Create a file named `Button.x.css`.

```css
.button {
    padding: 0.5rem 1rem;
    background-color: #e2e8f0;
}

.primary {
    background-color: #bfdbfe;
}
```

Import the stylesheet and pass the styles you want to `props()`.

```jsx
import { props } from "@csx-style/core";

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

When `primary` is true, the later style overrides the background color and the button keeps its padding. For plain JavaScript, assign `props(styles.button).className` to the element's `className`.

## TypeScript

Add these compiler options to the TypeScript config that covers your app code. If you already use `rootDirs`, add `.csx/types` alongside your existing roots. The paths below assume the config is in the app root.

```json
{
    "compilerOptions": {
        "allowArbitraryExtensions": true,
        "rootDirs": [".", ".csx/types"]
    }
}
```

Run your dev server and open the page that imports the `.x.css` file to generate its types. Add `.csx/types/` to `.gitignore`.

## Supported features

### Class selectors

Use a single class per selector, optionally with one of the pseudo-classes below. Comma-separated selector lists such as `.button, .link:focus-visible` are supported. Within each property tier and media group, layers run in order from base through `:hover`, `:focus`, `:focus-visible`, `:active`, `:disabled`, `:focus-within`, `:required`, and `:invalid`. Later layers take precedence.

### Media queries

Use top-level `@media` blocks to add responsive styles. Inside a block, you can use the same class selectors and pseudo-classes as at the top level.

```css
@media (min-width: 48rem) {
    .button {
        padding: 0.75rem 1.5rem;
    }
}
```

Keep using `props(styles.button)`. Styles composed with `props()` override each other per property, query, and selector state. In the generated CSS, media rules take precedence over unconditional rules within each reset, shorthand, or longhand tier. Overlapping queries follow stylesheet source order within the same cascade layer. When rules in that layer come from different files, CSX orders them by module ID.

### Theming

CSX is intended to be compatible and interoperable with vanilla CSS meaning that native features are heavily relied on for styling properly. Originally, CSX supported a custom StyleX-vars-style `@tokens` block, but that has been removed in favour of vanilla CSS variables (e.g. `:root { --foo: bar}` / `.button { color: var(--foo) }`) with potential future plans for bringing better precedence and automatic overriding behaviour via compiled style tags and group var classes.

This means that today, you should be defining your theme/style system in the `:root` block of your main CSS file, preferably before the `@csx` directive. Then just reference them as you would in normal css. You may also define/override variables within a CSX class, as variable definitions are treated the same way normal properties are in the compiler.

## Examples

- [Next.js App Router](https://github.com/Floffah/csx/tree/main/examples/next)
- [Vite with React](https://github.com/Floffah/csx/tree/main/examples/vite-react)
- [Vite with plain JavaScript](https://github.com/Floffah/csx/tree/main/examples/vite-js)
