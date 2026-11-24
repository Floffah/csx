import changelog from "@changesets/cli/changelog";

export const { getReleaseLine } = changelog;

export function getDependencyReleaseLine(changesets, dependenciesUpdated) {
    if (dependenciesUpdated.length === 0) return "";

    const commits = new Set(
        changesets.flatMap(({ commit }) =>
            commit ? [commit.slice(0, 7)] : [],
        ),
    );
    const heading = commits.size
        ? `- Updated dependencies [${[...commits].join(", ")}]`
        : "- Updated dependencies";
    const dependencies = dependenciesUpdated.map(
        ({ name, newVersion }) => `  - ${name}@${newVersion}`,
    );
    return [heading, ...dependencies].join("\n");
}
