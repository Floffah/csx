import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { setTimeout } from "node:timers/promises";
import postcss from "@csx-style/postcss";
import csx from "@csx-style/vite";
import { expect, test } from "bun:test";
import { createServer } from "vite";

test("keeps CSX modules and PostCSS output current during development", async () => {
    const root = await mkdtemp(join(tmpdir(), "csx-vite-"));
    const stylesheet = join(root, "unimported.x.css");
    const imported = join(root, "imported.x.css");
    await writeFile(join(root, "app.css"), "@csx;\n");
    await writeFile(imported, ".example { color: red }");
    await writeFile(
        join(root, "main.js"),
        'export { default } from "./imported.x.css";',
    );
    const ready = Promise.withResolvers<void>();
    const server = await createServer({
        root,
        configFile: false,
        plugins: [
            csx(),
            {
                name: "wait-for-watcher",
                configureServer(server) {
                    server.watcher.once("ready", ready.resolve);
                },
            },
        ],
        css: { postcss: { plugins: [postcss({ root })] } },
        server: {
            middlewareMode: true,
            ws: false,
            watch: { usePolling: true, interval: 25 },
        },
    });

    const css = async () =>
        (await server.transformRequest("/app.css?direct"))?.code ?? "";
    const waitForCss = async (value: string, present: boolean) => {
        for (let attempt = 0; attempt < 100; attempt++) {
            if ((await css()).includes(value) === present) return;
            await setTimeout(25);
        }
        expect((await css()).includes(value)).toBe(present);
    };

    try {
        await ready.promise;
        expect(await css()).not.toContain("@csx");
        const before = (await server.ssrLoadModule("/main.js")).default.example
            .color;
        expect(await css()).toContain(`.${before}`);
        await writeFile(stylesheet, ".preview { width: 321px }");
        await waitForCss("321px", true);
        await writeFile(stylesheet, ".preview { width: 654px }");
        await waitForCss("654px", true);
        expect(await css()).not.toContain("321px");
        await rm(stylesheet);
        await waitForCss("654px", false);

        await writeFile(imported, ".example { color: green }");
        await waitForCss("color: green", true);
        const after = (await server.ssrLoadModule("/main.js")).default.example
            .color;
        expect(after).not.toBe(before);
        expect(await css()).toContain(`.${after}`);
    } finally {
        await server.close();
        await rm(root, { recursive: true, force: true });
    }
}, 10_000);
