import { clsx } from "clsx";

type Style = Readonly<Record<string, string>>;

export function props(...styles: (Style | false | null | undefined)[]) {
    const classes = new Map<string, string>();

    for (const style of styles) {
        if (!style) continue;
        for (const [key, className] of Object.entries(style)) {
            classes.set(key, className);
        }
    }

    return { className: clsx([...classes.values()]) };
}
