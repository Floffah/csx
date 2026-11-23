import csx from "@csx-style/vite";
import { defineConfig } from "vite";

export default defineConfig({
    plugins: [csx()],
    build: {
        minify: false,
        modulePreload: false,
    },
});
