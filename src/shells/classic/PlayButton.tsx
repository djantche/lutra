import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { LaunchRing } from "../../components/LaunchRing/LaunchRing";
import { useGameActions } from "../../hooks/useGameActions";
import { launchLabel } from "../../lib/launch";
import { useLibrary } from "../../state/library";
import type { Game } from "../../types/game";

export function PlayButton({ game, size = "md" }: { game: Game; size?: "md" | "lg" }) {
  const { t } = useTranslation();
  const { running, launching, launch, label, toggle } = useGameActions(game.id);

  return (
    <Button
      // Always over artwork (the game's banner), where only the glass style reads in both themes.
      variant={running ? "glass" : "primary"}
      size={size}
      icon={launching ? undefined : running ? "stop" : "play"}
      onClick={toggle}
    >
      {launch && <LaunchRing step={launch.step} steps={launch.steps} />}
      {launching ? launchLabel(t, launch) : label}
    </Button>
  );
}

export function FavoriteButton({ game, size = "md" }: { game: Game; size?: "md" | "lg" }) {
  const { t } = useTranslation();
  const { toggleFavorite } = useLibrary();
  const label = game.favorite ? t("game.removeFavorite") : t("game.addFavorite");

  return (
    <Button
      variant="glass"
      size={size}
      icon={game.favorite ? "starFilled" : "star"}
      aria-pressed={game.favorite}
      aria-label={label}
      title={label}
      onClick={() => toggleFavorite(game.id)}
    />
  );
}
