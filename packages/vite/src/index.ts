import { randomUUID } from "node:crypto";
import {
    mkdir,
    readFile,
    realpath,
    rename,
    rm,
    writeFile,
} from "node:fs/promises";
import { basename, dirname, isAbsolute, join, relative, sep } from "node:path";
import {
    Compiler,
    emitMappedDeclarations,
    emitModule,
} from "@csx-style/compiler";
import postcss from "postcss";
import { normalizePath, type Environment, type Plugin } from "vite";

const modulePrefix = "\0csx:";
const moduleSuffix = ".js";

function sourceId(moduleId: string): string {
    return moduleId.slice(modulePrefix.length, -moduleSuffix.length);
}

export default function csx(): Plugin {
    const directiveStylesheets = new WeakMap<Environment, Set<string>>();

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

            const file = sourceId(moduleId);
            this.addWatchFile(file);
            const id = normalizePath(await realpath(file));
            const source = await readFile(file, "utf8");
            const compiler = new Compiler({
                resolve: async (specifier, importer) => {
                    const resolved = await this.resolve(specifier, importer);
                    if (!resolved || resolved.external) {
                        throw new Error(
                            `Could not resolve "${specifier}" from "${importer}"`,
                        );
                    }
                    return resolved.id;
                },
                load: (id) => readFile(id, "utf8"),
            });

            let result;
            try {
                result = compiler.compileModule({ id, source });
            } finally {
                compiler.free();
            }
            const root = this.environment.config.root;
            const stylesheet = relative(root, file);
            if (!isAbsolute(stylesheet) && !stylesheet.startsWith(`..${sep}`)) {
                const declarationPath = join(
                    root,
                    ".csx/types",
                    stylesheet.replace(/\.css$/, ".d.css.ts"),
                );
                const directory = dirname(declarationPath);
                const { declaration, map } = emitMappedDeclarations(result, {
                    file: basename(declarationPath),
                    source: normalizePath(relative(directory, file)),
                });
                await mkdir(directory, { recursive: true });
                const mapPath = `${declarationPath}.map`;
                const suffix = `.${randomUUID()}.tmp`;
                const temporaryDeclaration = declarationPath + suffix;
                const temporaryMap = mapPath + suffix;
                try {
                    await writeFile(temporaryDeclaration, declaration);
                    await writeFile(temporaryMap, map);
                    await rename(temporaryMap, mapPath);
                    await rename(temporaryDeclaration, declarationPath);
                } catch (error) {
                    await Promise.all([
                        rm(temporaryDeclaration, { force: true }),
                        rm(temporaryMap, { force: true }),
                    ]);
                    throw error;
                }
            }

            return emitModule(result);
        },
        transform(source, id) {
            const environment = this.environment;
            if (environment.mode !== "dev") return;
            const [file] = id.split("?", 1);
            if (!file.endsWith(".css") || file.endsWith(".x.css")) return;

            let stylesheets = directiveStylesheets.get(environment);
            stylesheets?.delete(id);
            if (!source.includes("@csx")) return;

            postcss
                .parse(source, { from: file })
                .walkAtRules("csx", (directive) => {
                    if (!directive.params && !directive.nodes) {
                        if (!stylesheets) {
                            stylesheets = new Set();
                            directiveStylesheets.set(environment, stylesheets);
                        }
                        stylesheets.add(id);
                        return false;
                    }
                });
        },
        hotUpdate: {
            order: "post",
            handler({ file, modules, type }) {
                if (!file.endsWith(".x.css")) return;

                const graph = this.environment.moduleGraph;
                const affected = new Set(modules);
                const imported = graph.getModuleById(
                    modulePrefix + file + moduleSuffix,
                );
                if (imported) affected.add(imported);

                // Vite expands PostCSS directory dependencies into existing files.
                if (type !== "update") {
                    for (const id of directiveStylesheets.get(
                        this.environment,
                    ) ?? []) {
                        const stylesheet = graph.getModuleById(id);
                        if (stylesheet) affected.add(stylesheet);
                    }
                }
                if (affected.size > modules.length) return [...affected];
            },
        },
    };
}
