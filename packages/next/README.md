# @csx-style/next

Use `withCsx` in a Next.js 16 App Router project with the default Turbopack bundler. Run `next dev` and `next build` as usual. The adapter requires Node.js 20.19 or later on the 20.x line, or Node.js 22.12 or later.

```js
// next.config.mjs
import withCsx from "@csx-style/next";

export default withCsx();
```

Install `@csx-style/core` for `props()` and `@csx-style/postcss` to expand `@csx;`. Add the PostCSS plugin to your project config.

```js
// postcss.config.mjs
export default {
    plugins: {
        "@csx-style/postcss": {},
    },
};
```

Add `@csx;` to a regular stylesheet and import that stylesheet from the root layout. From a Server or Client Component, import `./Button.x.css` and pass its exported styles to `props()`. Keep writing standard CSS in `.x.css` files so editors can use their built-in CSS support.

For TypeScript imports, add these options to your `tsconfig.json`. The loader generates mapped declarations in `.csx/types` as stylesheets are imported. Add `.csx/types/` to `.gitignore`.

```json
{
    "compilerOptions": {
        "allowArbitraryExtensions": true,
        "rootDirs": [".", ".csx/types"]
    }
}
```

Run Next.js from the app directory. See the [App Router example](../../examples/next) for a complete setup.
