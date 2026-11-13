import type { IconName } from "../../../components/Icon/Icon";
import type { ProfileSection } from "./ProfileEditor";

/** The pages of a console's (or game's) settings, in order. */
export const PROFILE_SECTIONS: { value: ProfileSection; icon: IconName }[] = [
  { value: "video", icon: "monitor" },
  { value: "controls", icon: "gamepad" },
  { value: "emulator", icon: "console" },
];
