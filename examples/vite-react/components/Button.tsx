import { props } from "@csx-style/core";
import React, { type ReactNode } from "react";

import styles from "./Button.x.css";

export function Button({
    children,
    onClick,
    disabled = false,
    secondary = false,
}: {
    children: ReactNode;
    onClick: () => void;
    disabled?: boolean;
    secondary?: boolean;
}) {
    return (
        <button
            type="button"
            onClick={onClick}
            disabled={disabled}
            {...props(styles.button, secondary && styles.secondary)}
        >
            {children}
        </button>
    );
}
