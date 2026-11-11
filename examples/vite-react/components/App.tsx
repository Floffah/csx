import { props } from "@csx/csx";
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
            <Button onClick={() => setCount((current) => current + 1)}>
                Increment
            </Button>
        </main>
    );
}
