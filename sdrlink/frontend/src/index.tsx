import { render } from "solid-js/web";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import "./styles.css";
import App from "./App";

const root = document.getElementById("app");
if (!root) throw new Error("#app not found");

const theme = localStorage.getItem("sdrlink.theme");
if (theme) document.documentElement.setAttribute("data-theme", theme);

render(() => <App />, root);
