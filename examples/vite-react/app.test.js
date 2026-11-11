import { readFile, realpath } from "node:fs/promises";
import { Compiler } from "@csx/compiler";
import { expect, spyOn, test } from "bun:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createServer } from "vite";

test("reuses one compiler for the React app's CSX imports", async () => {
    const server = await createServer({
        root: import.meta.dir,
        server: { middlewareMode: true, ws: false },
    });
    const compileModule = spyOn(Compiler.prototype, "compileModule");

    try {
        const { App } = await server.ssrLoadModule("/app.jsx");
        expect(compileModule).toHaveBeenCalledTimes(2);
        for (const file of ["app.x.css", "theme.x.css"]) {
            const id = await realpath(new URL(file, import.meta.url));
            const source = await readFile(id, "utf8");
            expect(compileModule).toHaveBeenCalledWith({ id, source });
        }
        const [first, second] = compileModule.mock.contexts;
        expect(first).toBeInstanceOf(Compiler);
        expect(second).toBe(first);
        const exports = compileModule.mock.results.map(
            ({ value }) => value.exports,
        );
        expect(exports[0].example.color).toMatch(/^c[0-9a-f]{7}$/);
        expect(exports[1].theme["background-color"]).toMatch(/^c[0-9a-f]{7}$/);
        expect(renderToStaticMarkup(createElement(App))).toBe(
            renderToStaticMarkup(
                createElement("pre", null, JSON.stringify(exports)),
            ),
        );
    } finally {
        compileModule.mockRestore();
        await server.close();
    }
});
