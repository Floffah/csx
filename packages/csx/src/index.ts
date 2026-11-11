import { clsx } from "clsx";

export type CSXStyle = Readonly<Record<string, string>>;

export function props(...styles: (CSXStyle | false | null | undefined)[]) {
    const classes = new Map<string, string>();

    for (const style of styles) {
        if (!style) continue;
        for (const [key, className] of Object.entries(style)) {
            classes.set(key, className);
        }
    }

    return { className: clsx([...classes.values()]) };
}
