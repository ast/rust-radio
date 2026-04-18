import type { Theme, ThemeTokens } from "./types";

/** Camel-case token name → kebab-cased CSS custom property name. */
const cssVar = (key: string): string =>
  "--color-" + key.replace(/[A-Z]/g, (c) => "-" + c.toLowerCase());

/** Apply the theme's tokens to document.documentElement. Call once at startup. */
export function applyTheme(theme: Theme): void {
  const root = document.documentElement;
  for (const [key, value] of Object.entries(theme.tokens) as [keyof ThemeTokens, string][]) {
    root.style.setProperty(cssVar(key), value);
  }
}
