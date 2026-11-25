import {
    Compiler,
    emitDeclarations,
    emitMappedDeclarations,
    emitModule,
    type CompileModuleOptions,
    type CompileModuleResult,
    type CompilerOptions,
    type Diagnostic,
} from "@csx-style/compiler";
import { decode } from "@jridgewell/sourcemap-codec";
import { expect, test } from "bun:test";

test("exports hashed class properties", () => {
    const options: CompilerOptions = {
        resolve: async () => "",
        load: async () => "",
    };
    const compiler = new Compiler(options);

    try {
        const input: CompileModuleOptions = {
            id: "/src/styles.x.css",
            source: `
            .button {
                --accent: red;
                color: var(--accent)
            }
            .link, .label {
                color: var(--accent)
            }
            .button:hover {
                color: blue
            }
            `,
        };
        const result: CompileModuleResult = compiler.compileModule(input);

        expect(result.diagnostics).toEqual([]);

        expect(result).toMatchSnapshot();

        expect(emitModule(result)).toBe(
            emitModule({ exports: result.exports }),
        );
        expect(emitDeclarations(result)).toBe(
            [
                'import type { CSXStyle } from "@csx-style/core";',
                "",
                "declare const styles: {",
                '    readonly "button": CSXStyle;',
                '    readonly "label": CSXStyle;',
                '    readonly "link": CSXStyle;',
                "};",
                "export default styles;",
                "",
            ].join("\n"),
        );
        const color = result.exports.button.color;
        expect(compiler.emit()).toContain(
            `.${color} { color: var(--accent); }`,
        );
        expect(compiler.emit()).toContain(
            `.${result.exports.button["--accent"]} { --accent: red; }`,
        );
        expect(
            compiler.emit().match(new RegExp(`\\.${color} `, "g")),
        ).toHaveLength(1);
        const hover = result.exports.button[":hover:color"];
        expect(compiler.emit()).toContain(`.${hover}:hover { color: #00f; }`);
    } finally {
        compiler.free();
    }
});

test("maps exported classes to the start of their preferred CSS rule", () => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });
    try {
        const result = compiler.compileModule({
            id: "/src/Button.x.css",
            source: [
                ".button:hover, :hover.hover-only { color: red; }",
                "/* 🦊 */ .button,",
                ".link { color: blue; }",
                ".button { color: green; }",
            ].join("\r\n"),
        });
        const { declaration, map } = emitMappedDeclarations(result, {
            file: "Button.x.d.css.ts",
            source: "../../../src/Button.x.css",
        });

        expect(declaration).toEndWith(
            "//# sourceMappingURL=Button.x.d.css.ts.map\n",
        );
        const { sources, mappings } = JSON.parse(map);
        expect(sources).toEqual(["../../../src/Button.x.css"]);
        expect(decode(mappings)).toEqual([
            [],
            [],
            [],
            [[13, 0, 1, 9]],
            [[13, 0, 0, 0]],
            [[13, 0, 1, 9]],
        ]);
    } finally {
        compiler.free();
    }
});

test("keeps base and hover atoms distinct and orders their layers", () => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        const { exports } = compiler.compileModule({
            id: "/hover.x.css",
            source: `
                .button { color: red; }
                .button:hover, :hover.link { color: red; }
                .button:hover { color: blue; }
            `,
        });
        const base = exports.button.color;
        const hover = exports.button[":hover:color"];
        const linkHover = exports.link[":hover:color"];
        expect(linkHover).not.toBe(base);
        const css = compiler.emit();
        expect(css).toContain(`.${base} { color: red; }`);
        expect(css).toContain(`.${hover}:hover { color: #00f; }`);
        expect(css).toContain(`.${linkHover}:hover { color: red; }`);
        expect(
            css.indexOf("@layer csx.longhand.unconditional.base {"),
        ).toBeLessThan(
            css.indexOf("@layer csx.longhand.unconditional.hover {"),
        );
    } finally {
        compiler.free();
    }
});

test("emits selected modules in a stable cascade order", () => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        expect(compiler.emit()).toBe("");
        const { exports } = compiler.compileModule({
            id: "/button.x.css",
            source: `.button { margin-top: 8px; margin: 0; all: unset; color: red; }
                     .button { color: blue; }`,
        });
        const other = compiler.compileModule({
            id: "/other.x.css",
            source: ".other { color: green; }",
        });
        const css = compiler.emit();
        expect(compiler.emit(["/other.x.css", "/button.x.css"])).toBe(css);
        expect(compiler.emit(["/button.x.css", "/other.x.css"])).toBe(css);
        expect(css.indexOf("all: unset")).toBeLessThan(
            css.indexOf("margin: 0"),
        );
        expect(css.indexOf("margin: 0")).toBeLessThan(
            css.indexOf("margin-top: 8px"),
        );
        expect(css).toContain(`.${exports.button.color} { color: #00f; }`);
        expect(css).not.toContain("color: red");
        expect(compiler.emit(["/button.x.css"])).not.toContain(
            other.exports.other.color,
        );
        expect(compiler.emit([])).toBe("");
    } finally {
        compiler.free();
    }
});

test("exports base, media, and media hover declarations independently", () => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        const { exports, exportLocations } = compiler.compileModule({
            id: "/button.x.css",
            source: [
                "@media screen, print {",
                "    .button, .primary { margin: 1rem; }",
                "    .button:hover { margin: 1rem; }",
                "    .print-only { color: red; }",
                "}",
                ".button { margin: 0; margin-left: 2rem; }",
            ].join("\n"),
        });
        const [mediaKey] = Object.keys(exports.primary);
        const [hoverKey] = Object.keys(exports.button).filter(
            (key) =>
                key !== "margin" && key !== "margin-left" && key !== mediaKey,
        );
        const media = exports.button[mediaKey];
        const hover = exports.button[hoverKey];
        expect(exports.primary[mediaKey]).toBe(media);
        expect(new Set([exports.button.margin, media, hover]).size).toBe(3);
        expect(exportLocations.button).toEqual({ line: 5, column: 0 });
        expect(exportLocations["print-only"]).toEqual({ line: 3, column: 4 });

        const css = compiler.emit();
        expect(css).toContain(
            `@media screen, print {\n    .${media} { margin: 1rem; }`,
        );
        expect(css).toContain(`.${hover}:hover { margin: 1rem; }`);
        expect(css.indexOf(`.${exports.button.margin} {`)).toBeLessThan(
            css.indexOf(`.${media} {`),
        );
        expect(css.indexOf(`.${hover}:hover {`)).toBeLessThan(
            css.indexOf(`.${exports.button["margin-left"]} {`),
        );
    } finally {
        compiler.free();
    }
});

test("keeps overlapping media rules in source order across repeated atoms", () => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        const id = "/button.x.css";
        const { exports } = compiler.compileModule({
            id,
            source: `
                @media (min-width: 48rem) { .early, .button { color: red; } }
                @media (min-width: 64rem) { .wide { color: blue; } }
                @media (min-width: 48rem) {
                    .late { color: red; }
                    .button { color: green; }
                }
            `,
        });
        const [queryKey] = Object.keys(exports.early);
        const early = exports.early[queryKey];
        const [wide] = Object.values(exports.wide);
        const late = exports.late[queryKey];
        const button = exports.button[queryKey];
        expect(Object.keys(exports.button)).toEqual([queryKey]);
        expect(late).not.toBe(early);
        expect(button).not.toBe(early);

        const otherId = "/other.x.css";
        compiler.compileModule({
            id: otherId,
            source: "@media (min-width: 48rem) { .other { color: red; } }",
        });
        expect(compiler.emit([id, otherId])).toBe(compiler.emit([otherId, id]));
        const css = compiler.emit([id]);
        expect(css.indexOf(`.${early} {`)).toBeLessThan(
            css.indexOf(`.${wide} {`),
        );
        expect(css.indexOf(`.${wide} {`)).toBeLessThan(
            css.indexOf(`.${late} {`),
        );
        expect(css).toContain(`.${button} { color: green; }`);
    } finally {
        compiler.free();
    }
});

test.each([
    ".button { color: red !important; }",
    ".button:hover { color: red !IMPORTANT; }",
    "@media (min-width: 500px) { .button { color: red !important; } }",
    ".button { --accent: red !important; }",
])("rejects important declarations in %s", (source) => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        expect(() =>
            compiler.compileModule({ id: "/button.x.css", source }),
        ).toThrow("!important is not supported in CSX stylesheets");
    } finally {
        compiler.free();
    }
});

test.each([
    ["button { color: red; }", "Unsupported selector `button`"],
    [
        '.button, [data-name="link"] { color: red; }',
        "Unsupported selector `[data-name=",
    ],
    [
        ".button:hover:focus { color: red; }",
        "Unsupported selector `.button:hover:focus`",
    ],
    [
        ".button { color: red; &:hover { color: blue; } }",
        "Nested rules are not supported",
    ],
    [
        "@media (min-width: 500px) { @media (hover: hover) { .button { color: red; } } }",
        "Unsupported rule `@media`",
    ],
    [
        "@media print { button { color: red; } }",
        "Unsupported selector `button`",
    ],
    ['@import "theme.css";', "Unsupported rule `@import`"],
    ["@unknown;", "Unsupported rule `@unknown`"],
    ["@tokens palette { --color: red; }", "Unsupported rule `@tokens`"],
])("rejects unsupported syntax in %s", (source, message) => {
    const compiler = new Compiler({
        resolve: async () => "",
        load: async () => "",
    });

    try {
        expect(() =>
            compiler.compileModule({ id: "/button.x.css", source }),
        ).toThrow(message);
    } finally {
        compiler.free();
    }
});
