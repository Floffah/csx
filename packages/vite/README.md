# @csx-style/vite

Use CSX in an existing Vite app with Vite 8.3.7 or later in the 8.x line.

## Add CSX to Vite

```sh
npm install @csx-style/core
npm install -D @csx-style/vite @csx-style/postcss postcss
```

Add `csx()` to the plugins in your Vite config.

```js
// vite.config.mjs
import csx from "@csx-style/vite";
import { defineConfig } from "vite";

export default defineConfig({ plugins: [csx()] });
```

## Add the PostCSS plugin

Add the plugin to your existing PostCSS config or create one in your app directory.

```js
// postcss.config.mjs
export default {
    plugins: {
        "@csx-style/postcss": {},
    },
};
```

Add `@csx;` to a regular stylesheet. If your entry point already imports a global stylesheet, add it there.

```css
/* styles.css */
@csx;
```

Import the stylesheet once from your app entry point.

```js
import "./styles.css";
```

## Use a CSX style

Create `Button.x.css`.

```css
.button {
    padding: 0.5rem 1rem;
    background-color: #e2e8f0;
}
```

Import the stylesheet in your component and pass the style to `props()`.

```jsx
import { props } from "@csx-style/core";

import styles from "./Button.x.css";

export function Button() {
    return (
        <button type="button" {...props(styles.button)}>
            Save
        </button>
    );
}
```

For plain JavaScript, assign `props(styles.button).className` to the element's `className`.

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

See the [React example](https://github.com/Floffah/csx/tree/main/examples/vite-react) or the [plain JavaScript example](https://github.com/Floffah/csx/tree/main/examples/vite-js) for a complete app.
