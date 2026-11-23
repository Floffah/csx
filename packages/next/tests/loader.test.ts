import { readFile } from "node:fs";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { expect, test } from "bun:test";

import loader from "../src/loader";

test("rewrites runtime CSX imports and re-exports without changing surrounding code", async () => {
    const root = await mkdtemp(join(tmpdir(), "csx-next-loader-"));
    const transform = (filename: string, source: string) =>
        new Promise<string>((done, reject) => {
            loader.call(
                {
                    resourcePath: join(root, filename),
                    rootContext: root,
                    sourceMap: false,
                    fs: { readFile },
                    async: () => (error, code) => {
                        if (error) reject(error);
                        else done(code!);
                    },
                    addDependency() {},
                    getResolve: () => async (directory, request) =>
                        resolve(directory, request),
                },
                source,
            );
        });

    try {
        await writeFile(join(root, "button.x.css"), ".button { color: red }");
        const source = String.raw`// ${"🦊"}
import styles from './button.x.css';
import { type Style } from './types.x.css';
import './globals.css';
export { default as first, default as second } from './button.x.css';
export type { Style } from './types.x.css';
const lazy = import(('./button\x2ex.css'));
const computed = import('./other.x.css' + suffix);
const jsx = <button className={styles.button.color} />;`;
        const output = await transform("page.tsx", source);
        const urls =
            output.match(/data:text\/javascript;base64,[\w+/=]+/g) ?? [];
        expect(urls).toHaveLength(3);
        expect(new Set(urls).size).toBe(1);
        expect(output).toContain("// 🦊\nimport styles from");
        expect(output).toContain("import { type Style } from './types.x.css';");
        expect(output).toContain("export type { Style } from './types.x.css';");
        expect(output).toContain("import './globals.css';");
        expect(output).toContain("import('./other.x.css' + suffix)");
        expect(output).toContain("<button className={styles.button.color} />");
        expect(
            await transform("page.js", "import './button.x.css'; <button />;"),
        ).toContain("data:text/javascript;base64,");
    } finally {
        await rm(root, { recursive: true, force: true });
    }
});
