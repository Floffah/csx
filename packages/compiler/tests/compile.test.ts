import {
    Compiler,
    type CompileModuleOptions,
    type CompileModuleResult,
    type CompilerOptions,
    type Diagnostic,
} from "@csx/compiler";
import { expect, test } from "bun:test";

test("returns the module result contract for the empty compiler stub", () => {
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
                --blue: #0000FF;
            }

            .button {
                color: var(--test-red)
            }
            `,
        };
        const result: CompileModuleResult = compiler.compileModule(input);

        expect(result.diagnostics).toEqual([]);

        // expect(result).toEqual({
        //     exports: {},
        //     variables: {},
        //     dependencies: [],
        //     diagnostics: [],
        // });
    } finally {
        compiler.free();
    }
});
