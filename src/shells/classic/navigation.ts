import type { LibraryFilter } from "../../lib/library";
import type { SettingsSection } from "../../views/SettingsView";

export type Page =
  | { name: "home" }
  | { name: "library"; filter: LibraryFilter }
  | { name: "game"; id: string }
  | { name: "settings"; section?: SettingsSection }
  | { name: "controllers" }
  /** The emulation dashboard, or one system's page. */
  | { name: "emulation"; system?: string };

export function pageKey(page: Page): string {
  switch (page.name) {
    case "library":
      return `library:${page.filter}`;
    case "game":
      return `game:${page.id}`;
    case "settings":
      return `settings:${page.section ?? ""}`;
    case "emulation":
      return `emulation:${page.system ?? ""}`;
    default:
      return page.name;
  }
}
