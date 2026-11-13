import { useId, useMemo, useState } from "react";
import { ART_HEIGHT, ART_WIDTH, createScene } from "../../lib/art";
import { cx } from "../../lib/cx";
import { useArtworkUrl } from "../../lib/useArtworkUrl";
import type { Game } from "../../types/game";
import { GameLogo } from "../GameLogo/GameLogo";
import styles from "./GameArt.module.scss";

interface GameArtProps {
  game: Game;
  /** `poster` is portrait box art, `hero` a wide banner. Generated art stands in when no image exists. */
  kind?: "poster" | "hero";
  /** Overlays the title as a logo, box-art style. */
  showTitle?: boolean;
  className?: string;
}

/** The procedural landscape standing in for a game's real artwork. */
function GeneratedArt({ game }: { game: Game }) {
  const scene = useMemo(() => createScene(game.id, game.hue), [game.id, game.hue]);
  const uid = useId().replace(/[^a-zA-Z0-9]/g, "");
  const ref = (name: string) => `${uid}-${name}`;
  const { sun } = scene;

  return (
    <svg viewBox={`0 0 ${ART_WIDTH} ${ART_HEIGHT}`} preserveAspectRatio="xMidYMid slice" focusable="false">
      <defs>
        <linearGradient id={ref("sky")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={scene.sky[0]} />
          <stop offset="0.55" stopColor={scene.sky[1]} />
          <stop offset="1" stopColor={scene.sky[2]} />
        </linearGradient>
        <radialGradient id={ref("glow")}>
          <stop offset="0" stopColor={sun.color} stopOpacity="0.75" />
          <stop offset="1" stopColor={sun.color} stopOpacity="0" />
        </radialGradient>
        <linearGradient id={ref("sun")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={sun.color} />
          <stop offset="1" stopColor={sun.colorLow} />
        </linearGradient>
        <linearGradient id={ref("fog")} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor={scene.sky[2]} stopOpacity="0" />
          <stop offset="1" stopColor={scene.sky[2]} stopOpacity="0.3" />
        </linearGradient>
      </defs>

      <rect width={ART_WIDTH} height={ART_HEIGHT} fill={`url(#${ref("sky")})`} />
      {scene.stars.map((star, index) => (
        <circle key={index} cx={star.x} cy={star.y} r={star.r} fill="#fff" opacity={star.opacity} />
      ))}
      <circle cx={sun.cx} cy={sun.cy} r={sun.r * 2.8} fill={`url(#${ref("glow")})`} />
      <circle cx={sun.cx} cy={sun.cy} r={sun.r} fill={`url(#${ref("sun")})`} />
      {scene.ring && (
        <ellipse
          cx={sun.cx}
          cy={sun.cy}
          rx={sun.r * 1.9}
          ry={sun.r * 0.4}
          fill="none"
          stroke={sun.color}
          strokeOpacity="0.6"
          strokeWidth="10"
          transform={`rotate(-14 ${sun.cx} ${sun.cy})`}
        />
      )}
      {scene.ridges.map((ridge, index) => (
        <g key={index}>
          <rect y={ridge.base - 260} width={ART_WIDTH} height={260} fill={`url(#${ref("fog")})`} />
          <path d={ridge.d} fill={ridge.fill} />
        </g>
      ))}
    </svg>
  );
}

/**
 * A game's cover or banner filling its container (the container decides the aspect ratio).
 * Generated art stands in until the real image is loaded, then is not drawn at all.
 */
export function GameArt({ game, kind = "poster", showTitle = false, className }: GameArtProps) {
  const image = useArtworkUrl(kind === "hero" ? game.hero : game.poster, game.artVersion);
  // An image already in memory shows at once, without the generated art or a fade.
  const [instant] = useState(image !== null);
  const [shown, setShown] = useState<string | null>(image);
  const covered = image !== null && shown === image;

  return (
    <div className={cx(styles.art, className)} aria-hidden="true">
      {!covered && <GeneratedArt game={game} />}

      {image && (
        <img
          className={cx(styles.image, !instant && styles.fadeIn)}
          src={image}
          alt=""
          draggable={false}
          onLoad={() => setShown(image)}
        />
      )}

      {showTitle && !image && (
        <div className={styles.title}>
          <GameLogo game={game} />
        </div>
      )}
    </div>
  );
}
