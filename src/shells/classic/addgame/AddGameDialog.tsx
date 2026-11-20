import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/Button/Button";
import { Dialog } from "../../../components/Dialog/Dialog";
import {
  detailsFromMeta,
  EMPTY_DETAILS,
  isValidDate,
  metaFromDetails,
  type DetailsForm,
} from "../../../components/GameDetailsFields/details";
import { NEW_PREFIX } from "../../../components/PrefixPicker/PrefixPicker";
import { Spinner } from "../../../components/Spinner/Spinner";
import { Stepper } from "../../../components/Stepper/Stepper";
import { useGameLookup } from "../../../hooks/useGameLookup";
import {
  cancelInstall,
  defaultRunner,
  listRunners,
  onInstallFinished,
  pickLinuxProgram,
  pickProgram,
  setDefaultRunner,
  startInstall,
  type GameHit,
  type InstallFinished,
  type Runner,
} from "../../../lib/backend";
import { onRunnersChanged } from "../../../lib/runners";
import { useLibraryStore } from "../../../state/library";
import { reportError } from "../../../state/toasts";
import { ArtworkStep } from "./ArtworkStep";
import { DetailsStep } from "./DetailsStep";
import { InstallRunStep } from "./InstallRunStep";
import { LauncherStep } from "./LauncherStep";
import { SourceStep, type Kind } from "./SourceStep";
import styles from "./AddGame.module.scss";

type Step = "source" | "details" | "launcher" | "install" | "artwork";

/** The steps of each kind of game. The game is added when leaving the one before the images. */
const FLOWS: Record<Kind, Step[]> = {
  windows: ["source", "details", "launcher", "artwork"],
  linux: ["source", "details", "artwork"],
  install: ["source", "launcher", "install", "details", "artwork"],
};

/** "C:\Games\Foo Bar\foo_bar.exe" -> "foo_bar". */
function fileStem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  return name.replace(/\.[^.]+$/, "");
}

interface AddGameDialogProps {
  onClose: () => void;
  onAdded: (id: string) => void;
}

/**
 * Adds a game in steps, each one small: the program, the details, the launcher, then, once the
 * game is in the library, its images. A bar and the numbered steps show where the user is.
 */
export function AddGameDialog({ onClose, onAdded }: AddGameDialogProps) {
  const { t } = useTranslation();
  const addGame = useLibraryStore((state) => state.addGame);
  const removeGame = useLibraryStore((state) => state.removeGame);

  const [kind, setKind] = useState<Kind>("windows");
  const [step, setStep] = useState<Step>("source");
  const [exePath, setExePath] = useState("");
  const [title, setTitle] = useState("");
  const [titleEdited, setTitleEdited] = useState(false);
  const [details, setDetails] = useState<DetailsForm>(EMPTY_DETAILS);
  const [matchId, setMatchId] = useState<number | null>(null);
  const [runners, setRunners] = useState<Runner[] | null>(null);
  const [preferred, setPreferred] = useState<string | null>(null);
  const [runnerPath, setRunnerPath] = useState("");
  const [makeDefault, setMakeDefault] = useState(false);
  const [prefixPath, setPrefixPath] = useState(NEW_PREFIX);
  const [prefixName, setPrefixName] = useState("");
  const [prefixNameEdited, setPrefixNameEdited] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Installing: the installer runs, then the program it installed is chosen.
  const [install, setInstall] = useState<{ installId: string; prefixPath: string } | null>(null);
  const [installResult, setInstallResult] = useState<InstallFinished | null>(null);
  const [installExe, setInstallExe] = useState("");
  // Once added, the game is in the library and only its images are left.
  const [addedId, setAddedId] = useState<string | null>(null);
  const added = useLibraryStore((state) => (addedId ? (state.games.find((game) => game.id === addedId) ?? null) : null));

  useEffect(() => {
    const load = () =>
      Promise.all([listRunners(), defaultRunner()])
        .then(([found, wanted]) => {
          setRunners(found);
          const stillThere = found.find((runner) => runner.path === wanted)?.path ?? null;
          setPreferred(stillThere);
          // The runner chosen in settings, when it is still installed; otherwise the first one found.
          setRunnerPath((current) => found.find((runner) => runner.path === current)?.path ?? stillThere ?? found[0]?.path ?? "");
        })
        .catch(() => setRunners([]));
    void load();
    // A runner installed from this window (or elsewhere) shows up without reopening it.
    const stop = onRunnersChanged(() => void load());
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  useEffect(() => {
    if (!install) return;
    const stop = onInstallFinished((event) => {
      if (event.installId !== install.installId) return;
      setInstallResult(event);
      // The likeliest program is already selected; the user confirms or changes it.
      setInstallExe(event.candidates[0]?.path ?? "");
    });
    return () => void stop.then((unlisten) => unlisten());
  }, [install]);

  // GameDB is searched by what the user calls the game, only while they are on that step.
  const lookup = useGameLookup(step === "details" ? title : "");

  const flow = FLOWS[kind];
  const index = flow.indexOf(step);
  const runner = runners?.find((candidate) => candidate.path === runnerPath);
  const needsRunner = kind !== "linux";
  const dateValid = isValidDate(details.releaseDate);
  /** The step after which the game is added. */
  const addsHere = flow[index + 1] === "artwork";

  // A new prefix is called after the game until the user says otherwise.
  const newPrefixName = (prefixNameEdited ? prefixName : title).trim() || fileStem(exePath);

  const canNext: Record<Step, boolean> = {
    source: exePath !== "",
    details: title.trim() !== "" && dateValid,
    launcher: runner !== undefined,
    install: installResult !== null && installExe !== "",
    artwork: true,
  };

  const browse = () => {
    (kind === "linux" ? pickLinuxProgram() : pickProgram())
      .then((picked) => {
        if (!picked) return;
        setExePath(picked);
        if (!titleEdited) setTitle(fileStem(picked));
      })
      .catch((reason) => setError(String(reason)));
  };

  const chooseKind = (next: Kind) => {
    if (next === kind) return;
    setKind(next);
    // A program of the other kind cannot be kept.
    setExePath("");
    setError(null);
  };

  const useMatch = (hit: GameHit) => {
    setMatchId(hit.id);
    setTitle(hit.name);
    setTitleEdited(true);
    setDetails(detailsFromMeta(hit.meta));
  };

  const failed = (reason: unknown) => {
    setError(t("addGame.failed", { error: String(reason) }));
    setBusy(false);
  };

  const create = () => {
    if (needsRunner && !runner) return;
    const program = kind === "install" ? installExe : exePath;
    setBusy(true);
    setError(null);
    addGame({
      title: title.trim(),
      exePath: program,
      runnerKind: needsRunner ? (runner?.kind ?? "wine") : "native",
      runnerPath: needsRunner ? (runner?.path ?? "") : "",
      prefixPath:
        kind === "install"
          ? installResult?.prefixPath || install?.prefixPath
          : needsRunner && prefixPath !== NEW_PREFIX
            ? prefixPath
            : undefined,
      prefixName: kind !== "install" && needsRunner && prefixPath === NEW_PREFIX ? newPrefixName : undefined,
      meta: metaFromDetails(details),
    })
      .then((game) => {
        setAddedId(game.id);
        setStep("artwork");
        setBusy(false);
      })
      .catch(failed);
  };

  const startInstaller = () => {
    if (!runner) return;
    setBusy(true);
    setError(null);
    startInstall({
      installerPath: exePath,
      title: title.trim() || fileStem(exePath),
      runnerKind: runner.kind,
      runnerPath: runner.path,
      prefixPath: prefixPath !== NEW_PREFIX ? prefixPath : undefined,
      prefixName: prefixPath === NEW_PREFIX ? newPrefixName : undefined,
    })
      .then((started) => {
        setInstall(started);
        setInstallResult(null);
        setInstallExe("");
        setStep("install");
        setBusy(false);
      })
      .catch(failed);
  };

  const next = () => {
    if (!canNext[step] || busy) return;
    setError(null);
    if (step === "launcher" && makeDefault && runner) {
      setDefaultRunner(runner.path)
        .then(() => setPreferred(runner.path))
        .catch((reason) => reportError(t("addGame.defaultFailed"), reason));
    }
    if (step === "source" && kind === "install" && title === "") setTitle(fileStem(exePath));
    if (step === "launcher" && kind === "install") return startInstaller();
    if (addsHere) return create();
    setStep(flow[index + 1]);
  };

  /** The installer is stopped when the user leaves its step or closes the window. */
  const stopInstaller = () => {
    if (install && !installResult) cancelInstall(install.installId).catch((reason) => reportError(t("install.cancelFailed"), reason));
  };

  const back = () => {
    if (index === 0 || busy) return;
    if (step === "artwork" && addedId) {
      // The game is already in the library: take it out again so that it is added once, with
      // whatever the user changes. Nothing else exists yet (the prefix is made at the first launch).
      setBusy(true);
      removeGame(addedId, false)
        .then(() => {
          setAddedId(null);
          setBusy(false);
          setStep(flow[index - 1]);
        })
        .catch((reason) => {
          reportError(t("addGame.backFailed"), reason);
          setBusy(false);
        });
      return;
    }
    if (step === "install") {
      stopInstaller();
      setInstall(null);
      setInstallResult(null);
    }
    setError(null);
    setStep(flow[index - 1]);
  };

  const close = () => {
    stopInstaller();
    onClose();
  };

  const finish = () => {
    onClose();
    if (addedId) onAdded(addedId);
  };

  const labels: Record<Step, string> = {
    source: t("addGame.steps.source.label"),
    details: t("addGame.steps.details.label"),
    launcher: t("addGame.steps.launcher.label"),
    install: t("addGame.steps.install.label"),
    artwork: t("addGame.steps.artwork.label"),
  };

  const primary =
    step === "artwork"
      ? t("addGame.finish")
      : step === "launcher" && kind === "install"
        ? t("install.start")
        : addsHere
          ? t("addGame.add")
          : t("addGame.next");

  return (
    <Dialog
      size="xl"
      title={t("addGame.title")}
      onClose={step === "artwork" ? finish : close}
      busy={busy}
      footer={
        <>
          {step !== "artwork" && (
            <Button variant="ghost" onClick={close} disabled={busy}>
              {t("addGame.cancel")}
            </Button>
          )}
          <span style={{ flex: 1 }} />
          {index > 0 && (
            <Button variant="secondary" icon="arrowLeft" onClick={back} disabled={busy}>
              {t("common.back")}
            </Button>
          )}
          <Button variant="primary" disabled={!canNext[step] || busy} onClick={step === "artwork" ? finish : next}>
            {busy && <Spinner />}
            {busy && addsHere ? t("addGame.adding") : primary}
          </Button>
        </>
      }
    >
      <div className={styles.wizard}>
        <Stepper steps={flow.map((id) => ({ id, label: labels[id] }))} current={Math.max(0, index)} />

        <div className={styles.stage} key={step}>
          {step === "source" && <SourceStep kind={kind} onKind={chooseKind} exePath={exePath} onBrowse={browse} />}
          {step === "details" && (
            <DetailsStep
              title={title}
              onTitle={(value) => {
                setTitle(value);
                setTitleEdited(true);
                // Typing something else means the match no longer describes this game.
                setMatchId(null);
              }}
              details={details}
              onDetails={setDetails}
              lookup={lookup}
              matchId={matchId}
              onMatch={useMatch}
            />
          )}
          {step === "launcher" && (
            <LauncherStep
              runners={runners}
              runnerPath={runnerPath}
              onRunner={setRunnerPath}
              defaultPath={preferred}
              makeDefault={makeDefault}
              onMakeDefault={setMakeDefault}
              prefixPath={prefixPath}
              onPrefix={setPrefixPath}
              prefixName={prefixNameEdited ? prefixName : title.trim() || fileStem(exePath)}
              onPrefixName={(value) => {
                setPrefixName(value);
                setPrefixNameEdited(true);
              }}
              installing={kind === "install"}
            />
          )}
          {step === "install" && <InstallRunStep finished={installResult} exePath={installExe} onExe={setInstallExe} />}
          {step === "artwork" && <ArtworkStep game={added} />}

          {error && <p className={styles.error}>{error}</p>}
        </div>
      </div>
    </Dialog>
  );
}
