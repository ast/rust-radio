import { render } from "solid-js/web";
import App from "./App";
import { applyTheme, theme } from "./theme";
import "./styles/index.css";

applyTheme(theme);

const root = document.getElementById("app");

if (!root) {
  throw new Error("Root element not found");
}

render(() => <App />, root);
