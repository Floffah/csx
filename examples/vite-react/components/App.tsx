import { props } from "@csx-style/core";
import React, { useState } from "react";

import styles from "./App.x.css";
import { Button } from "./Button";

export function App() {
    const [count, setCount] = useState(0);

    return (
        <main {...props(styles.card)}>
            <h1 {...props(styles.title)}>Counter</h1>
            <output {...props(styles.count, count > 0 && styles.counted)}>
                {count}
            </output>
            <div {...props(styles.actions)}>
                <Button onClick={() => setCount((current) => current + 1)}>
                    Increment
                </Button>
                <Button
                    secondary
                    disabled={count === 0}
                    onClick={() => setCount(0)}
                >
                    Reset
                </Button>
            </div>
        </main>
    );
}
