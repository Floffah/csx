# @csx-style/postcss

Expands `@csx;` into atomic CSS from the project's `*.x.css` files.

```js
// postcss.config.mjs
export default {
    plugins: {
        "@csx-style/postcss": {},
    },
};
```

Put the directive in a regular stylesheet that your app loads.

```css
@csx;
```

The plugin scans the working directory by default. Set `root` in the plugin options to scan a different directory, such as an app within a monorepo. Relative paths resolve from the working directory.

The plugin reports file and directory dependencies to the PostCSS runner. Your build tool uses those dependencies to decide when to reprocess the stylesheet. Component imports of `*.x.css` and TypeScript declarations are handled by the bundler adapter.
