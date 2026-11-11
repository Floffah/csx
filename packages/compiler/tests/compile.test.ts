import {
    Compiler,
    emitModule,
    type CompileModuleOptions,
    type CompileModuleResult,
    type CompilerOptions,
    type Diagnostic,
} from "@csx/compiler";
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
            @tokens test {
                --red: #FF0000;
                --blue: #0000FF
            }

            .button {
                color: var(--test-red)
            }
            .link, .label {
                color: var(--test-red)
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
        const color = result.exports.button.color;
        expect(compiler.emit()).toContain(
            `.${color} { color: var(--test-red); }`,
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
                .button:hover:focus { color: green; }
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
        expect(css).not.toContain("color: green");
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

test.each([
    ".button { color: red !important; }",
    ".button:hover { color: red !IMPORTANT; }",
    "@media (min-width: 500px) { .button { color: red !important; } }",
    "@tokens palette { --color: red !important; }",
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
