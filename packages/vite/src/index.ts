import { randomUUID } from "node:crypto";
import { mkdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { basename, dirname, isAbsolute, join, relative, sep } from "node:path";
import { Compiler, emitMappedDeclarations, emitModule } from "@csx/compiler";
import postcss, { type AtRule } from "postcss";
import { normalizePath, type Environment, type Plugin } from "vite";

const modulePrefix = "\0csx:";
const moduleSuffix = ".js";
const manifestName = "virtual:csx-manifest";
const manifestId = "\0" + manifestName;

function sourceId(moduleId: string): string {
    return moduleId.slice(modulePrefix.length, -moduleSuffix.length);
}

export default function csx(): Plugin {
    const compilers = new WeakMap<Environment, Compiler>();
    const directiveStylesheets = new WeakMap<Environment, Set<string>>();

    return {
        name: "csx",
        enforce: "pre",
        async resolveId(source, importer) {
            if (source === manifestName) return manifestId;
            if (!source.endsWith(".x.css") || source.startsWith("\0")) return;

            const resolved = await this.resolve(source, importer);
            if (!resolved || resolved.external) return;

            // Use a JS module ID so Vite skips its CSS transforms.
            return modulePrefix + resolved.id + moduleSuffix;
        },
        async load(moduleId) {
            if (moduleId === manifestId) {
                return `
                    export default import.meta.glob("/**/*.x.css", { eager: true });
                    if (import.meta.hot) import.meta.hot.accept();
                `;
            }
            if (!moduleId.startsWith(modulePrefix)) return;

            const id = sourceId(moduleId);
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

            const result = compiler.compileModule({ id, source });
            const root = this.environment.config.root;
            const stylesheet = relative(root, id);
            if (!isAbsolute(stylesheet) && !stylesheet.startsWith(`..${sep}`)) {
                const declarationPath = join(
                    root,
                    ".csx/types",
                    stylesheet.replace(/\.css$/, ".d.css.ts"),
                );
                const directory = dirname(declarationPath);
                const { declaration, map } = emitMappedDeclarations(result, {
                    file: basename(declarationPath),
                    source: normalizePath(relative(directory, id)),
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
        async transform(source, id) {
            const [file] = id.split("?", 1);
            if (!file.endsWith(".css") || file.endsWith(".x.css")) return;

            directiveStylesheets.get(this.environment)?.delete(id);
            if (!source.includes("@csx")) return;

            const stylesheet = postcss.parse(source, { from: file });
            const directives: AtRule[] = [];
            stylesheet.walkAtRules("csx", (directive) => {
                if (!directive.params && !directive.nodes) {
                    directives.push(directive);
                }
            });
            if (directives.length === 0) return;

            let moduleIds: string[];
            const environment = this.environment;
            if (environment.mode === "dev") {
                let ids = directiveStylesheets.get(this.environment);
                if (!ids) {
                    ids = new Set();
                    directiveStylesheets.set(this.environment, ids);
                }
                ids.add(id);
                await environment.transformRequest(manifestName);
                const manifest =
                    environment.moduleGraph.getModuleById(manifestId);
                const modules = [...(manifest?.importedModules ?? [])].filter(
                    (module) => module.id?.startsWith(modulePrefix),
                );
                // Transforming the manifest resolves its imports without loading them.
                await Promise.all(
                    modules.map((module) =>
                        environment.transformRequest(module.url),
                    ),
                );
                moduleIds = modules.map((module) => sourceId(module.id!));
            } else {
                const manifest = await this.load({
                    id: manifestId,
                    resolveDependencies: true,
                });
                const modules = manifest.importedIds.filter((id) =>
                    id.startsWith(modulePrefix),
                );
                await Promise.all(modules.map((id) => this.load({ id })));
                moduleIds = modules.map(sourceId);
            }

            const css = compilers.get(this.environment)?.emit(moduleIds) ?? "";
            for (const directive of directives) {
                directive.replaceWith(css);
            }
            const result = stylesheet.toResult({
                to: file,
                map: { inline: false, annotation: false },
            });
            return { code: result.css, map: result.map.toString() };
        },
        hotUpdate: {
            order: "post",
            handler({ file, modules, timestamp, type }) {
                const graph = this.environment.moduleGraph;
                const manifest = graph.getModuleById(manifestId);
                const updated =
                    type === "update" && file.endsWith(".x.css")
                        ? graph.getModuleById(
                              modulePrefix + file + moduleSuffix,
                          )
                        : undefined;
                const membershipChanged =
                    manifest && modules.includes(manifest);
                const affected = new Set(modules);
                if (updated) affected.add(updated);
                if (
                    !membershipChanged &&
                    !(updated && manifest?.importedModules.has(updated))
                ) {
                    return updated ? [...affected] : undefined;
                }
                if (membershipChanged) {
                    // Vite flagged the glob; only its stylesheet consumers need HMR.
                    graph.invalidateModule(
                        manifest,
                        undefined,
                        timestamp,
                        true,
                    );
                    affected.delete(manifest);
                }
                for (const id of directiveStylesheets.get(this.environment) ??
                    []) {
                    const stylesheet = graph.getModuleById(id);
                    if (stylesheet) affected.add(stylesheet);
                }
                return [...affected];
            },
        },
        closeBundle() {
            compilers.get(this.environment)?.free();
            compilers.delete(this.environment);
        },
    };
}
