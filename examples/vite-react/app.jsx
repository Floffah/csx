import React from "react";

import "./app.css";
import styles from "./app.x.css";
import theme from "./theme.x.css";

export function App() {
    return <pre>{JSON.stringify([styles, theme])}</pre>;
}
