import type { NextConfig } from "next";

const sourceFiles = "*.{js,jsx,ts,tsx,mjs,cjs,mts,cts}";

export default function withCsx(config: NextConfig = {}): NextConfig {
    const existing = config.turbopack?.rules?.[sourceFiles];
    return {
        ...config,
        turbopack: {
            ...config.turbopack,
            rules: {
                ...config.turbopack?.rules,
                [sourceFiles]: [
                    ...(existing ? [existing].flat() : []),
                    {
                        condition: {
                            all: [
                                { not: "foreign" },
                                { content: /\.x\.css|\\[ux]/ },
                            ],
                        },
                        loaders: [require.resolve("./loader.cjs")],
                    },
                ],
            },
        },
    };
}
