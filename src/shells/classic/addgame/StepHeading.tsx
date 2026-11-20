import { Icon, type IconName } from "../../../components/Icon/Icon";
import styles from "./AddGame.module.scss";

/** The title of a step of the "add a game" flow, with what it is for. */
export function StepHeading({ icon, title, hint }: { icon: IconName; title: string; hint: string }) {
  return (
    <header className={styles.heading}>
      <span className={styles.headingIcon} aria-hidden="true">
        <Icon name={icon} size={24} />
      </span>
      <div>
        <h3>{title}</h3>
        <p>{hint}</p>
      </div>
    </header>
  );
}
