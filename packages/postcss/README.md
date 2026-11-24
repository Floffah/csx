See the [CSX homepage](https://github.com/Floffah/csx) for full setup instructions. You must integrate CSX with your build tool before using this plugin.

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
