import { props } from "@csx/csx";
import React, { type ReactNode } from "react";

import styles from "./Button.x.css";

export function Button({
    children,
    onClick,
}: {
    children: ReactNode;
    onClick: () => void;
}) {
    return (
        <button type="button" onClick={onClick} {...props(styles.button)}>
            {children}
        </button>
    );
}
