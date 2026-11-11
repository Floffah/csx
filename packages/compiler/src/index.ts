import "../wasm/csx_compiler.js";

import { encode, type SourceMapMappings } from "@jridgewell/sourcemap-codec";

import type { CompileModuleResult } from "../wasm/csx_compiler.js";

export { Compiler } from "../wasm/csx_compiler.js";
export type {
    CompilerOptions,
    CompileModuleOptions,
    CompileModuleResult,
    Diagnostic,
    DiagnosticSeverity,
} from "../wasm/csx_compiler.js";

export function emitModule({
    exports,
}: Pick<CompileModuleResult, "exports">): string {
    return `export default ${JSON.stringify(exports)};`;
}

export function emitDeclarations({
    exports,
}: Pick<CompileModuleResult, "exports">): string {
    return declareStyles(Object.keys(exports).sort());
}

const declarationHeader = [
    'import type { CSXStyle } from "@csx/csx";',
    "",
    "declare const styles: {",
];
const propertyPrefix = "    readonly ";

function declareStyles(names: string[]): string {
    return [
        ...declarationHeader,
        ...names.map(
            (name) => `${propertyPrefix}${JSON.stringify(name)}: CSXStyle;`,
        ),
        "};",
        "export default styles;",
        "",
    ].join("\n");
}

export function emitMappedDeclarations(
    {
        exports,
        exportLocations,
    }: Pick<CompileModuleResult, "exports" | "exportLocations">,
    { file, source }: { file: string; source: string },
): { declaration: string; map: string } {
    const names = Object.keys(exports).sort();
    const mappings: SourceMapMappings = declarationHeader.map(() => []);
    for (const name of names) {
        const location = Object.hasOwn(exportLocations, name)
            ? exportLocations[name]
            : undefined;
        mappings.push(
            location
                ? [[propertyPrefix.length, 0, location.line, location.column]]
                : [],
        );
    }

    return {
        declaration: `${declareStyles(names)}//# sourceMappingURL=${encodeURIComponent(file)}.map\n`,
        map: JSON.stringify({
            version: 3,
            file,
            sources: [source],
            names: [],
            mappings: encode(mappings),
        }),
    };
}
