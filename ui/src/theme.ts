import { ref } from "vue";

/** Operator colour-scheme choice; `system` follows the OS (`prefers-color-scheme`). */
export type ThemeChoice = "light" | "dark" | "system";

const THEME_KEY = "kowalski.ui.theme.v1";

function readStored(): ThemeChoice {
  try {
    const v = localStorage.getItem(THEME_KEY);
    if (v === "light" || v === "dark" || v === "system") return v;
  } catch {
    /* storage may be unavailable (private window, blocked site data) */
  }
  return "system";
}

function apply(choice: ThemeChoice) {
  const root = document.documentElement;
  if (choice === "system") root.removeAttribute("data-theme");
  else root.setAttribute("data-theme", choice);
}

export const themeChoice = ref<ThemeChoice>(readStored());

/** `?theme=light|dark|system` overrides the stored choice for this page load. */
export function initTheme() {
  const q = new URLSearchParams(window.location.search).get("theme");
  if (q === "light" || q === "dark" || q === "system") themeChoice.value = q;
  apply(themeChoice.value);
}

export function setTheme(choice: ThemeChoice) {
  themeChoice.value = choice;
  apply(choice);
  try {
    localStorage.setItem(THEME_KEY, choice);
  } catch {
    /* ignore */
  }
}
