import { create } from "zustand";
import type { InstallEvent, InstallStage } from "../lib/runners";

/** Log lines kept per install. */
const MAX_LINES = 400;

export interface InstallJob {
  name: string;
  stage: InstallStage;
  received: number;
  total: number;
  /** Bytes per second while downloading, smoothed. */
  speed: number;
  log: string[];
  /** Set once the job ended, whatever the way. */
  finished: boolean;
  sample: { at: number; received: number } | null;
}

interface InstallsState {
  jobs: Record<string, InstallJob>;
  apply: (event: InstallEvent) => void;
  /** Shows an install that was already running when the page opened. */
  adopt: (name: string) => void;
  dismiss: (name: string) => void;
}

const ENDED: InstallStage[] = ["done", "failed", "cancelled"];

const blank = (name: string): InstallJob => ({
  name,
  stage: "prepare",
  received: 0,
  total: 0,
  speed: 0,
  log: [],
  finished: false,
  sample: null,
});

export const useRunnerInstalls = create<InstallsState>((set) => ({
  jobs: {},
  apply: (event) =>
    set((state) => {
      // A new run of the same install starts from a clean slate.
      const previous = state.jobs[event.name];
      const job = previous && !previous.finished ? previous : blank(event.name);
      const now = performance.now();
      let { speed, sample } = job;
      if (event.stage === "download") {
        if (sample && now - sample.at >= 400 && event.received >= sample.received) {
          const instant = ((event.received - sample.received) * 1000) / (now - sample.at);
          speed = speed === 0 ? instant : speed * 0.6 + instant * 0.4;
          sample = { at: now, received: event.received };
        } else if (!sample) {
          sample = { at: now, received: event.received };
        }
      } else {
        speed = 0;
        sample = null;
      }
      const log = event.line ? [...job.log, event.line].slice(-MAX_LINES) : job.log;
      return {
        jobs: {
          ...state.jobs,
          [event.name]: {
            ...job,
            stage: event.stage,
            received: event.received,
            total: event.total,
            speed,
            sample,
            log,
            finished: ENDED.includes(event.stage),
          },
        },
      };
    }),
  adopt: (name) =>
    set((state) => (state.jobs[name] ? state : { jobs: { ...state.jobs, [name]: blank(name) } })),
  dismiss: (name) =>
    set((state) => {
      const { [name]: _gone, ...rest } = state.jobs;
      return { jobs: rest };
    }),
}));

/** Share of the current stage that is done, or null when the stage has no measurable progress. */
export function jobProgress(job: InstallJob): number | null {
  if (job.stage === "done") return 1;
  if ((job.stage === "download" || job.stage === "extract") && job.total > 0) {
    return Math.min(1, job.received / job.total);
  }
  return null;
}
