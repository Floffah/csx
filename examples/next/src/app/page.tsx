import { props } from "@csx-style/core";

import styles from "./page.x.css";

export default function Home() {
    return <h1 {...props(styles.heading)}>CSX with Next.js</h1>;
}
