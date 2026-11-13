import { create } from "zustand";
import { logFrontend } from "../lib/backend";

export interface Toast {
  id: number;
  kind: "error" | "info";
  text: string;
}

const LIFETIME_MS = 6000;
let nextId = 1;

interface ToastState {
  toasts: Toast[];
  notify: (kind: Toast["kind"], text: string) => void;
  dismiss: (id: number) => void;
}

export const useToastStore = create<ToastState>((set, get) => ({
  toasts: [],
  notify: (kind, text) => {
    const id = nextId++;
    set((state) => ({ toasts: [...state.toasts, { id, kind, text }] }));
    window.setTimeout(() => get().dismiss(id), LIFETIME_MS);
  },
  dismiss: (id) => set((state) => ({ toasts: state.toasts.filter((toast) => toast.id !== id) })),
}));

/** Shows a failure to the user, whatever the backend or the browser threw. */
export function reportError(context: string, error: unknown): void {
  console.error(context, error);
  const detail = error instanceof Error ? error.message : String(error);
  useToastStore.getState().notify("error", `${context} ${detail}`);
  void logFrontend("error", `${context} ${detail}`).catch(() => {});
}
