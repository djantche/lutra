import { siEpicgames, siGogdotcom, siLinux, siSteam, siWine } from "simple-icons";
import { SOURCE_COLOR } from "../../lib/sources";
import type { GameSource } from "../../types/game";

// Logos from Simple Icons (CC0), https://simpleicons.org
const LOGOS: Record<GameSource, string> = {
  steam: siSteam.path,
  gog: siGogdotcom.path,
  epic: siEpicgames.path,
  wine: siWine.path,
  native: siLinux.path,
};

/** The logo of a game source, in its color. */
export function SourceIcon({ source, size = 18 }: { source: GameSource; size?: number }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      fill={SOURCE_COLOR[source]}
      aria-hidden="true"
      style={{ flex: "none" }}
    >
      <path d={LOGOS[source]} />
    </svg>
  );
}
