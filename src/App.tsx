import { useEffect } from "react";
import type { InputAction } from "./input/actions";
import { emitAction } from "./input/bus";
import { useInputAction } from "./input/useInputAction";
import { onControllerAction } from "./lib/controllers";
import { useKeyboardInput } from "./input/useKeyboardInput";
import { BigPictureShell } from "./shells/bigpicture/BigPictureShell";
import { ClassicShell } from "./shells/classic/ClassicShell";
import { RunnerInstallEffects } from "./state/RunnerInstallEffects";
import { BatteryAlerts } from "./state/BatteryAlerts";
import { UpdateEffects } from "./state/UpdateEffects";
import { LibraryEffects } from "./state/LibraryEffects";
import { PowerDialog } from "./components/PowerDialog/PowerDialog";
import { UiProvider, useUi } from "./state/ui";

function Shell() {
  const { mode, setMode, toggleMode, powerMenuOpen, setPowerMenuOpen } = useUi();

  useKeyboardInput();

  // The webview's own right-click menu (Reload, Inspect...) is never wanted. Text fields keep
  // theirs for copy and paste; everything else opens the app's menu, or nothing.
  useEffect(() => {
    const onContextMenu = (event: MouseEvent) => {
      const target = event.target;
      const editable =
        target instanceof HTMLElement && (target.matches("input:not([type=button]), textarea") || target.isContentEditable);
      if (!editable) event.preventDefault();
    };
    window.addEventListener("contextmenu", onContextMenu);
    return () => window.removeEventListener("contextmenu", onContextMenu);
  }, []);

  // The controller driver sends the same actions as the keyboard while no game runs.
  useEffect(() => {
    const stop = onControllerAction((action) => emitAction(action as InputAction));
    return () => void stop.then((unlisten) => unlisten());
  }, []);
  useInputAction((action) => {
    if (action === "toggleMode") {
      toggleMode();
      return true;
    }
    // The Guide button: from the desktop it enters Big Picture (Big Picture's own menu handles it there).
    if (action === "menu" && mode === "classic") {
      setMode("bigpicture");
      return true;
    }
    // The Guide button held for a few seconds: the power menu.
    if (action === "powerMenu") {
      setPowerMenuOpen(true);
      return true;
    }
  });

  return (
    <>
      {mode === "bigpicture" ? <BigPictureShell /> : <ClassicShell />}
      {powerMenuOpen && <PowerDialog />}
    </>
  );
}

function App() {
  return (
    <UiProvider>
      <LibraryEffects />
      <BatteryAlerts />
      <UpdateEffects />
      <RunnerInstallEffects />
      <Shell />
    </UiProvider>
  );
}

export default App;
