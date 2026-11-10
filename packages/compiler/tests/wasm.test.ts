import { spawnSync } from "node:child_process";
import { expect, test } from "bun:test";

test.each(["node", "bun"])(
    "%s constructs the built WASM compiler",
    (runtime) => {
        const result = spawnSync(
            runtime,
            [
                "--input-type=module",
                "-e",
                `
    import { Compiler } from '@csx/compiler';
    const compiler = new Compiler({
      resolve: async () => '/theme.x.css',
      load: async () => '',
    });
    compiler.free();
  `,
            ],
            {
                cwd: new URL("../", import.meta.url),
                encoding: "utf8",
            },
        );

        expect(result.error).toBeUndefined();
        expect(result.stderr).toBe("");
        expect(result.status).toBe(0);
    },
);
