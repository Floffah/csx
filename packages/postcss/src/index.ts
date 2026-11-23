import { readFile, realpath } from "node:fs/promises";
import { dirname, resolve, sep } from "node:path";
import type { AtRule, Plugin } from "postcss";
import { glob } from "tinyglobby";

const pluginName = "@csx-style/postcss";
const sourceGlob = "**/*.x.css";

export interface Options {
    root?: string;
}

export default function csx({ root = process.cwd() }: Options = {}): Plugin {
    return {
        postcssPlugin: pluginName,
        async Once(stylesheet, { result }) {
            const parent = result.opts.from;
            if (parent?.endsWith(".x.css")) return;

            const directives: AtRule[] = [];
            stylesheet.walkAtRules("csx", (directive) => {
                if (!directive.params && !directive.nodes) {
                    directives.push(directive);
                }
            });
            if (directives.length === 0) return;

            const dir = await realpath(root);
            const files = await glob(sourceGlob, {
                cwd: dir,
                absolute: true,
                followSymbolicLinks: false,
                ignore: ["**/node_modules/**"],
            });
            result.messages.push({
                type: "dir-dependency",
                plugin: pluginName,
                dir,
                glob: sourceGlob,
                parent,
            });

            const { Compiler } = await import("@csx-style/compiler");
            const compiler = new Compiler({
                resolve: (specifier, importer) =>
                    realpath(resolve(dirname(importer), specifier)),
                load: (id) => readFile(id, "utf8"),
            });
            try {
                for (const file of files.sort()) {
                    const id = file.split(sep).join("/");
                    const source = await readFile(file, "utf8");
                    try {
                        compiler.compileModule({ id, source });
                    } catch (error) {
                        throw new Error(`${file}: ${String(error)}`, {
                            cause: error,
                        });
                    }
                    result.messages.push({
                        type: "dependency",
                        plugin: pluginName,
                        file: id,
                        parent,
                    });
                }

                const css = compiler.emit();
                for (const directive of directives) {
                    directive.replaceWith(css);
                }
            } finally {
                compiler.free();
            }
        },
    };
}

csx.postcss = true as const;
