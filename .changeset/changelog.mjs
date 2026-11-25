import githubChangelog from "@changesets/changelog-github";

export async function getReleaseLine(changeset, type, options) {
    // Put author metadata on its own line so filtering never touches the summary.
    const rendered = await githubChangelog.getReleaseLine(changeset, type, {
        ...options,
        template: "{authors}\n- {summary} {ref}",
    });
    const [authors, entry, ...continuation] = rendered.split("\n");
    const owner = options.repo.split("/")[0].toLowerCase();
    const contributors = authors
        .split(",")
        .map((author) => author.trim())
        .filter(
            (author) =>
                author && !author.toLowerCase().startsWith(`[@${owner}]`),
        );
    const credit = contributors.length
        ? ` Thanks ${contributors.join(", ")}!`
        : "";
    return `\n\n${[entry + credit, ...continuation].join("\n")}`;
}

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
