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
import MagicString from "magic-string";
import type { ValueSpan } from "oxc-parser";

interface LoaderContext {
    resourcePath: string;
    rootContext: string;
    sourceMap: boolean;
    fs: {
        readFile(
            file: string,
            encoding: "utf8",
            callback: (error: Error | null, source: string) => void,
        ): void;
    };
    async(): (error: Error | null, source?: string, map?: string) => void;
    addDependency(file: string): void;
    getResolve(): (directory: string, specifier: string) => Promise<string>;
}

export default function loader(this: LoaderContext, source: string) {
    const callback = this.async();
    transform.call(this, source).then(
        ({ code, map }) => callback(null, code, map),
        (error) => callback(error),
    );
}

async function transform(this: LoaderContext, source: string) {
    const { parseSync } = await import("oxc-parser");
    const parsed = parseSync(this.resourcePath, source, {
        sourceType: "unambiguous",
        lang: /\.[cm]?js$/.test(this.resourcePath) ? "jsx" : undefined,
    });
    const error = parsed.errors.find(({ severity }) => severity === "Error");
    if (error) throw new Error(error.codeframe ?? error.message);

    const imports = new Map<number, ValueSpan>();
    const addImport = (request: ValueSpan | null) => {
        if (request?.value.endsWith(".x.css"))
            imports.set(request.start, request);
    };
    const { staticImports, staticExports, dynamicImports } = parsed.module;
    for (const { moduleRequest, entries } of staticImports) {
        if (entries.length === 0 || entries.some(({ isType }) => !isType)) {
            addImport(moduleRequest);
        }
    }
    for (const { entries } of staticExports) {
        for (const { isType, moduleRequest } of entries) {
            if (!isType) addImport(moduleRequest);
        }
    }
    for (const {
        moduleRequest: { start, end },
    } of dynamicImports) {
        const argument = source.slice(start, end);
        if (!/\.x\.css|\\[ux]/.test(argument)) continue;
        // Oxc exposes the span but only decodes static module specifiers.
        const [statement] = parseSync("import.js", argument, {
            preserveParens: false,
        }).program.body;
        if (statement?.type !== "ExpressionStatement") continue;
        const literal = statement.expression;
        if (literal.type === "Literal" && typeof literal.value === "string") {
            addImport({
                value: literal.value,
                start: start + literal.start,
                end: start + literal.end,
            });
        }
    }
    if (imports.size === 0) return { code: source };

    const resolve = this.getResolve();
    const resolveStyle = async (specifier: string, importer: string) =>
        realpath(await resolve(dirname(importer), specifier));
    const readStyle = (file: string) =>
        new Promise<string>((resolve, reject) => {
            this.fs.readFile(file, "utf8", (error, source) => {
                if (error) reject(error);
                else resolve(source);
            });
        });
    const { Compiler, emitMappedDeclarations, emitModule } =
        await import("@csx-style/compiler");
    const compiler = new Compiler({
        resolve: resolveStyle,
        load: readStyle,
    });
    const output = new MagicString(source);
    const modules = new Map<string, string>();

    try {
        const root = await realpath(this.rootContext);
        for (const { value, start, end } of imports.values()) {
            const file = await resolveStyle(value, this.resourcePath);
            let url = modules.get(file);
            if (url === undefined) {
                const result = compiler.compileModule({
                    id: normalizePath(file),
                    source: await readStyle(file),
                });
                url = `data:text/javascript;base64,${Buffer.from(emitModule(result)).toString("base64")}`;
                modules.set(file, url);

                const stylesheet = relative(root, file);
                if (
                    !isAbsolute(stylesheet) &&
                    !stylesheet.startsWith(`..${sep}`)
                ) {
                    const declarationPath = join(
                        root,
                        ".csx/types",
                        stylesheet.replace(/\.css$/, ".d.css.ts"),
                    );
                    await writeDeclarations(
                        declarationPath,
                        emitMappedDeclarations(result, {
                            file: basename(declarationPath),
                            source: normalizePath(
                                relative(dirname(declarationPath), file),
                            ),
                        }),
                    );
                    // A restored build cache must regenerate missing declarations.
                    this.addDependency(declarationPath);
                    this.addDependency(`${declarationPath}.map`);
                }
            }
            output.overwrite(start, end, JSON.stringify(url));
        }
    } finally {
        compiler.free();
    }

    return {
        code: output.toString(),
        map: this.sourceMap
            ? output
                  .generateMap({
                      source: this.resourcePath,
                      includeContent: true,
                      hires: "boundary",
                  })
                  .toString()
            : undefined,
    };
}

function normalizePath(file: string) {
    return file.split(sep).join("/");
}

async function writeDeclarations(
    path: string,
    { declaration, map }: { declaration: string; map: string },
) {
    const mapPath = `${path}.map`;
    const current = await Promise.all(
        [path, mapPath].map((file) =>
            readFile(file, "utf8").catch((error) => {
                if (error.code !== "ENOENT") throw error;
            }),
        ),
    );
    if (current[0] === declaration && current[1] === map) return;

    await mkdir(dirname(path), { recursive: true });
    const suffix = `.${randomUUID()}.tmp`;
    const temporaryDeclaration = path + suffix;
    const temporaryMap = mapPath + suffix;
    try {
        await writeFile(temporaryDeclaration, declaration);
        await writeFile(temporaryMap, map);
        await rename(temporaryMap, mapPath);
        await rename(temporaryDeclaration, path);
    } catch (error) {
        await Promise.all([
            rm(temporaryDeclaration, { force: true }),
            rm(temporaryMap, { force: true }),
        ]);
        throw error;
    }
}
