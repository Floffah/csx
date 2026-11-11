import { readFile } from "node:fs/promises";
import { Compiler, emitModule } from "@csx/compiler";
import type { Environment, Plugin } from "vite";

const modulePrefix = "\0csx:";
const moduleSuffix = ".js";

export default function csx(): Plugin {
    const compilers = new WeakMap<Environment, Compiler>();

    return {
        name: "csx",
        enforce: "pre",
        async resolveId(source, importer) {
            if (!source.endsWith(".x.css") || source.startsWith("\0")) return;

            const resolved = await this.resolve(source, importer);
            if (!resolved || resolved.external) return;

            // Use a JS module ID so Vite skips its CSS transforms.
            return modulePrefix + resolved.id + moduleSuffix;
        },
        async load(moduleId) {
            if (!moduleId.startsWith(modulePrefix)) return;

            const id = moduleId.slice(
                modulePrefix.length,
                -moduleSuffix.length,
            );
            this.addWatchFile(id);

            const source = await readFile(id, "utf8");
            let compiler = compilers.get(this.environment);
            if (!compiler) {
                compiler = new Compiler({
                    resolve: async (specifier, importer) => {
                        const resolved = await this.resolve(
                            specifier,
                            importer,
                        );
                        if (!resolved || resolved.external) {
                            throw new Error(
                                `Could not resolve "${specifier}" from "${importer}"`,
                            );
                        }
                        return resolved.id;
                    },
                    load: (id) => readFile(id, "utf8"),
                });
                compilers.set(this.environment, compiler);
            }

            return emitModule(compiler.compileModule({ id, source }));
        },
        closeBundle() {
            compilers.get(this.environment)?.free();
            compilers.delete(this.environment);
        },
    };
}
