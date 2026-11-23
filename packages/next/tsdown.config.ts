import { defineConfig } from "tsdown";

export default defineConfig({
    entry: ["src/index.ts", "src/loader.ts"],
    format: "cjs",
    fixedExtension: true,
    dts: true,
});
