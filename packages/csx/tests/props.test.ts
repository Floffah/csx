import * as csx from "@csx/csx";
import { expect, test } from "bun:test";

test("keeps the last atom for each property", () => {
    const base = Object.freeze({ color: "c1111111", margin: "c2222222" });
    const variant = { color: "c3333333" };
    const override = { color: "c4444444", "margin-top": "c5555555" };

    expect(csx.props(base, variant, override)).toEqual({
        className: "c4444444 c2222222 c5555555",
    });
});

test("composes base and hover atoms independently", () => {
    const button = { color: "c1111111", ":hover:color": "c2222222" };
    const danger = { color: "c3333333" };
    const dangerHover = { ":hover:color": "c4444444" };

    expect(csx.props(button, danger)).toEqual({
        className: "c3333333 c2222222",
    });
    expect(csx.props(button, danger, dangerHover)).toEqual({
        className: "c3333333 c4444444",
    });
});

test("ignores omitted styles", () => {
    expect(csx.props(undefined, { color: "c1111111" }, false, null)).toEqual({
        className: "c1111111",
    });
    expect(csx.props()).toEqual({ className: "" });
});
