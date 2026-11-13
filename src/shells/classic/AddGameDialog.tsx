import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/Button/Button";
import { Dialog } from "../../components/Dialog/Dialog";
import { FilePicker } from "../../components/FilePicker/FilePicker";
import { FormField } from "../../components/FormField/FormField";
import { GameDetailsFields } from "../../components/GameDetailsFields/GameDetailsFields";
import {
  detailsFromMeta,
  EMPTY_DETAILS,
  isValidDate,
  metaFromDetails,
  type DetailsForm,
} from "../../components/GameDetailsFields/details";
import { GameMatches } from "../../components/GameMatches/GameMatches";
import { InstallLatest } from "../../components/RunnerInstall/InstallLatest";
import { InstallStep, type InstallChoice } from "./InstallStep";
import { NEW_PREFIX, PrefixPicker } from "../../components/PrefixPicker/PrefixPicker";
import { SegmentedControl } from "../../components/SegmentedControl/SegmentedControl";
import { Select } from "../../components/Select/Select";
import { Spinner } from "../../components/Spinner/Spinner";
import { TextField } from "../../components/TextField/TextField";
import { useGameLookup } from "../../hooks/useGameLookup";
import {
  defaultRunner,
  listRunners,
  pickLinuxProgram,
  pickProgram,
  startInstall,
  type GameHit,
  type Runner,
} from "../../lib/backend";
import { onRunnersChanged } from "../../lib/runners";
import { useLibraryStore } from "../../state/library";
import styles from "./AddGameDialog.module.scss";

type Kind = "windows" | "linux" | "install";

/** "C:\Games\Foo Bar\foo_bar.exe" -> "foo_bar". */
function fileStem(path: string): string {
  const name = path.split(/[\\/]/).pop() ?? "";
  return name.replace(/\.[^.]+$/, "");
}

interface AddGameDialogProps {
  onClose: () => void;
  onAdded: (id: string) => void;
}

export function AddGameDialog({ onClose, onAdded }: AddGameDialogProps) {
  const { t } = useTranslation();
  const addGame = useLibraryStore((state) => state.addGame);

  const [kind, setKind] = useState<Kind>("windows");
  const [exePath, setExePath] = useState("");
  const [title, setTitle] = useState("");
  const [titleEdited, setTitleEdited] = useState(false);
  const [details, setDetails] = useState<DetailsForm>(EMPTY_DETAILS);
  const [matchId, setMatchId] = useState<number | null>(null);
  const [runners, setRunners] = useState<Runner[] | null>(null);
  const [runnerPath, setRunnerPath] = useState("");
  const [prefixPath, setPrefixPath] = useState(NEW_PREFIX);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Installing: the installer runs, then the game it installed is chosen.
  const [install, setInstall] = useState<{ installId: string; prefixPath: string } | null>(null);
  const [choice, setChoice] = useState<InstallChoice | null>(null);

  useEffect(() => {
    const load = () =>
      Promise.all([listRunners(), defaultRunner()])
        .then(([found, preferred]) => {
          setRunners(found);
          // The runner chosen in settings, when it is still installed; otherwise the first one found.
          setRunnerPath((current) =>
            found.find((runner) => runner.path === (current || preferred))?.path ?? found[0]?.path ?? "",
          );
        })
        .catch(() => setRunners([]));
    void load();
    // A runner installed from this window (or elsewhere) shows up without reopening it.
    const stop = onRunnersChanged(() => void load());
    return () => void stop.then((unlisten) => unlisten());
  }, []);

  // GameDB is searched by what the user calls the game: the name field, or the file name until they edit it.
  const lookup = useGameLookup(exePath === "" || kind === "install" ? "" : title);

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

  const runner = runners?.find((candidate) => candidate.path === runnerPath);
  const needsRunner = kind === "windows" || kind === "install";
  const installing = kind === "install";
  const dateValid = isValidDate(details.releaseDate);
  const canSubmit = exePath !== "" && (!needsRunner || runner !== undefined) && dateValid && !busy;
  const afterInstall = install !== null;

  const finish = (game: { id: string }) => {
    onClose();
    onAdded(game.id);
  };
  const failed = (reason: unknown) => {
    setError(t("addGame.failed", { error: String(reason) }));
    setBusy(false);
  };

  const submit = () => {
    if (!canSubmit) return;
    setBusy(true);
    setError(null);
    if (installing) {
      // Runs the setup program; the game is added once the installer is done.
      if (!runner || (runner.kind !== "wine" && runner.kind !== "proton")) return;
      startInstall({
        installerPath: exePath,
        title: title.trim(),
        runnerKind: runner.kind,
        runnerPath: runner.path,
        prefixPath: prefixPath !== NEW_PREFIX ? prefixPath : undefined,
      })
        .then((started) => {
          setInstall(started);
          setBusy(false);
        })
        .catch(failed);
      return;
    }
    addGame({
      title: title.trim(),
      exePath,
      runnerKind: needsRunner ? (runner?.kind ?? "wine") : "native",
      runnerPath: needsRunner ? (runner?.path ?? "") : "",
      prefixPath: needsRunner && prefixPath !== NEW_PREFIX ? prefixPath : undefined,
      meta: metaFromDetails(details),
    })
      .then(finish)
      .catch(failed);
  };

  const addInstalled = () => {
    if (!choice || !runner || !dateValid) return;
    setBusy(true);
    setError(null);
    addGame({
      title: choice.title,
      exePath: choice.exePath,
      runnerKind: runner.kind,
      runnerPath: runner.path,
      prefixPath: choice.prefixPath,
      meta: metaFromDetails(choice.details),
    })
      .then(finish)
      .catch(failed);
  };

  const runnerOptions = useMemo(
    () =>
      (runners ?? []).map((candidate) => ({
        value: candidate.path,
        label: candidate.label,
        hint: candidate.kind === "proton" ? "Proton" : "Wine",
      })),
    [runners],
  );

  return (
    <Dialog
      size="lg"
      title={t("addGame.title")}
      description={t("addGame.hint")}
      onClose={onClose}
      busy={busy}
      footer={
        <>
          <Button variant="ghost" onClick={onClose} disabled={busy}>
            {t("addGame.cancel")}
          </Button>
          {afterInstall ? (
            <Button variant="primary" disabled={!choice || busy} onClick={addInstalled}>
              {busy && <Spinner />}
              {busy ? t("addGame.adding") : t("addGame.add")}
            </Button>
          ) : (
            <Button variant="primary" disabled={!canSubmit} onClick={submit}>
              {busy && <Spinner />}
              {installing ? t("install.start") : busy ? t("addGame.adding") : t("addGame.add")}
            </Button>
          )}
        </>
      }
    >
      {afterInstall && runner && install ? (
        <InstallStep
          installId={install.installId}
          title={title.trim() || fileStem(exePath)}
          prefixPath={install.prefixPath}
          onChoice={setChoice}
          onBack={() => {
            setInstall(null);
            setChoice(null);
          }}
          busy={busy}
        />
      ) : (
      <>
      <SegmentedControl
        ariaLabel={t("addGame.kind")}
        autofocus
        segments={[
          { value: "windows", label: t("addGame.kindWindows"), icon: "settings" },
          { value: "linux", label: t("addGame.kindLinux"), icon: "terminal" },
          { value: "install", label: t("addGame.kindInstall"), icon: "plus" },
        ]}
        value={kind}
        onChange={chooseKind}
      />

      <FormField
        label={installing ? t("install.installer") : t("addGame.program")}
        hint={kind === "linux" ? t("addGame.linuxHint") : installing ? t("install.installerHint") : undefined}
      >
        <FilePicker value={exePath} placeholder={t("addGame.browse")} onPick={browse} />
      </FormField>

      {exePath !== "" && (
        <>
          <FormField label={t("addGame.name")}>
            <TextField
              value={title}
              placeholder={t("addGame.namePlaceholder")}
              onChange={(value) => {
                setTitle(value);
                setTitleEdited(true);
                // Typing something else means the match no longer describes this game.
                setMatchId(null);
              }}
            />
          </FormField>

          {!installing && (
            <>
              <div className={styles.section}>
                <span className={styles.sectionTitle}>{t("details.fromGameDb")}</span>
                <GameMatches {...lookup} selected={matchId} onSelect={useMatch} />
              </div>

              <GameDetailsFields value={details} onChange={setDetails} />
            </>
          )}
        </>
      )}

      {needsRunner && (
        <>
          <FormField label={t("addGame.runner")}>
            {runners !== null && runners.length === 0 ? (
              <>
                <p className={styles.warning}>{t("addGame.noRunner")}</p>
                <InstallLatest />
              </>
            ) : (
              <Select
                value={runnerPath}
                onChange={setRunnerPath}
                ariaLabel={t("addGame.runner")}
                options={runnerOptions}
              />
            )}
          </FormField>

          {runner && (
            <FormField label={t("config.prefix")}>
              <PrefixPicker
                allowNew
                ariaLabel={t("config.prefix")}
                runnerKind={runner.kind}
                value={prefixPath}
                onChange={setPrefixPath}
              />
            </FormField>
          )}
          <p className={styles.note}>{installing ? t("install.optimized") : t("addGame.optimized")}</p>
        </>
      )}
      </>
      )}

      {error && <p className={styles.error}>{error}</p>}
    </Dialog>
  );
}
