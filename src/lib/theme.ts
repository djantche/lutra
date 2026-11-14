// Light, dark, or whatever the system says. Kept in this browser and applied as the app starts;
// `styles/_tokens.scss` reads `data-theme` on <html> (no attribute: follow the system).

const THEME = "lutra.theme";

export type ThemeChoice = "system" | "light" | "dark";

export function theme(): ThemeChoice {
  try {
    const chosen = localStorage.getItem(THEME);
    return chosen === "light" || chosen === "dark" ? chosen : "system";
  } catch {
    return "system";
  }
}

/** Puts the saved choice to work. Called once as the interface starts. */
export function applyTheme(choice: ThemeChoice = theme()): void {
  const root = document.documentElement;
  if (choice === "system") delete root.dataset.theme;
  else root.dataset.theme = choice;
}

export function setTheme(choice: ThemeChoice): void {
  try {
    if (choice === "system") localStorage.removeItem(THEME);
    else localStorage.setItem(THEME, choice);
  } catch {
    // The choice only lasts for this session.
  }
  applyTheme(choice);
}
