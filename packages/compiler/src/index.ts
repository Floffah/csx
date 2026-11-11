import "../wasm/csx_compiler.js";

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
