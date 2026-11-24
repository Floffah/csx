import { readFile } from "node:fs/promises";
import { expect, test } from "bun:test";

test("the App Router build emits HTML, CSS, and declarations for a CSX import", async () => {
    const build = new URL("../.next/", import.meta.url);
    const html = await readFile(
        new URL("server/app/index.html", build),
        "utf8",
    );
    const classes =
        html.match(/<h1 class="([^"]+)">CSX with Next\.js<\/h1>/)?.[1] ?? "";
    expect(classes).toMatch(/^c[\da-f]{8}x c[\da-f]{8}x$/);

    const cssHref = html.match(/href="\/_next\/(static\/[^" ]+\.css)"/)?.[1];
    expect(cssHref).toBeDefined();
    const css = await readFile(new URL(cssHref ?? "", build), "utf8");
    for (const className of classes.split(" ")) {
        expect(css).toContain(`.${className}{`);
    }

    const declarationUrl = new URL(
        "../.csx/types/src/app/page.x.d.css.ts",
        import.meta.url,
    );
    const declaration = await readFile(declarationUrl, "utf8");
    expect(declaration).toContain('readonly "heading": CSXStyle;');
    const map = JSON.parse(
        await readFile(new URL(`${declarationUrl.href}.map`), "utf8"),
    );
    expect(
        map.sources.map(
            (source: string) => new URL(source, declarationUrl).href,
        ),
    ).toEqual([new URL("../src/app/page.x.css", import.meta.url).href]);
});
