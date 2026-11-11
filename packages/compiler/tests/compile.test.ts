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

        expect(emitModule(result)).toBe(emitModule({ exports: result.exports }));
    } finally {
        compiler.free();
    }
});
