import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import fr from "./locales/fr.json";

const STORAGE_KEY = "lutra.language";

export const LANGUAGES = [
  { code: "fr", label: "Français" },
  { code: "en", label: "English" },
] as const;

function initialLanguage(): string {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored) return stored;
  } catch {
    // Persistence is a convenience only.
  }
  return navigator.language.startsWith("fr") ? "fr" : "en";
}

i18n.use(initReactI18next).init({
  resources: { en: { translation: en }, fr: { translation: fr } },
  lng: initialLanguage(),
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

document.documentElement.lang = i18n.language;
i18n.on("languageChanged", (language) => {
  document.documentElement.lang = language;
});

export function setLanguage(code: string): void {
  void i18n.changeLanguage(code);
  try {
    localStorage.setItem(STORAGE_KEY, code);
  } catch {
    // Persistence is a convenience only.
  }
}

export default i18n;
