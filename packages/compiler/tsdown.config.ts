import { wasm } from "rolldown-plugin-wasm";
import { defineConfig, type UserConfig } from "tsdown";

const common = {
    entry: ["src/index.ts"],
    format: "esm",
    fixedExtension: true,
    dts: true,
} satisfies UserConfig;

export default defineConfig([
    {
        ...common,
        platform: "browser",
        outDir: `dist/browser`,
        plugins: [wasm({ targetEnv: "browser", maxFileSize: 0 })],
    },
    {
        ...common,
        platform: "node",
        outDir: `dist/node`,
        plugins: [wasm({ targetEnv: "node", maxFileSize: 0 })],
    },
]);
