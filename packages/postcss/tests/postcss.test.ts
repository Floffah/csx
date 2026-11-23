import { mkdir, mkdtemp, realpath, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Compiler } from "@csx-style/compiler";
import { expect, test } from "bun:test";
import postcss from "postcss";

import csx from "../src/index";

test("expands @csx from the current file set and reports dependencies", async () => {
    const dir = await realpath(await mkdtemp(join(tmpdir(), "csx-postcss-")));
    const from = join(dir, "globals.css");
    const nested = join(dir, "components");
    const first = join(dir, "first.x.css");
    const second = join(nested, "second.x.css");
    const firstSource = ".first { color: red }";
    const processor = postcss([csx({ root: dir })]);
    const process = () => processor.process("@csx;", { from });
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        const empty = await process();
        const directoryDependency = {
            type: "dir-dependency",
            plugin: "@csx-style/postcss",
            dir,
            glob: "**/*.x.css",
            parent: from,
        };
        expect(empty.css).toBe("");
        expect(empty.messages).toEqual([directoryDependency]);

        await mkdir(nested);
        await mkdir(join(dir, "node_modules"));
        await writeFile(join(dir, "node_modules", "ignored.x.css"), "}");
        await writeFile(first, firstSource);
        await writeFile(second, ".second { margin: 42px }");
        const initial = await process();
        const id = first.replaceAll("\\", "/");
        const { exports } = compiler.compileModule({ id, source: firstSource });
        expect(initial.css).toContain(
            `.${exports.first.color} { color: red; }`,
        );
        expect(initial.css).toContain("margin: 42px");
        expect(initial.messages).toEqual([
            directoryDependency,
            ...[second, first].map((file) => ({
                type: "dependency",
                plugin: "@csx-style/postcss",
                file: file.replaceAll("\\", "/"),
                parent: from,
            })),
        ]);

        const repeated = await processor.process("@csx;\n@csx;", { from });
        const origins: number[] = [];
        repeated.root.walkRules((rule) => {
            expect(rule.source?.input.file).toBe(from);
            origins.push(rule.source?.start?.line ?? 0);
        });
        expect(origins).toEqual([1, 1, 2, 2]);

        await writeFile(first, ".first { color: green }");
        await rm(second);
        const updated = await process();
        expect(updated.css).toContain("color: green");
        expect(updated.css).not.toContain(exports.first.color);
        expect(updated.css).not.toContain("margin: 42px");
        expect(updated.messages).toEqual([
            directoryDependency,
            initial.messages[2],
        ]);

        await writeFile(first, "}");
        await expect(process().async()).rejects.toThrow(first);
    } finally {
        compiler.free();
        await rm(dir, { recursive: true, force: true });
    }
});

test("only expands a bare @csx directive in regular stylesheets", async () => {
    const processor = postcss([csx({ root: "unused-source-directory" })]);
    for (const [css, from] of [
        ["/* @csx; */ @csx extra; @csx {}", "globals.css"],
        ["@csx;", "styles.x.css"],
    ]) {
        const result = await processor.process(css, { from });
        expect(result.css).toBe(css);
        expect(result.messages).toEqual([]);
    }
});
