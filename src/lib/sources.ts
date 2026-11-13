import type { GameSource } from "../types/game";

export const SOURCES: GameSource[] = ["steam", "gog", "epic", "wine", "native"];

export const SOURCE_COLOR: Record<GameSource, string> = {
  steam: "#66c0f4",
  gog: "#b45fe0",
  epic: "#d6d8e6",
  wine: "#e0566f",
  native: "#3ddc97",
};
