import { create } from "zustand";
import type { IconName } from "../components/Icon/Icon";

export type MenuItem =
  | {
      type?: "item";
      label: string;
      icon?: IconName;
      onSelect: () => void;
      danger?: boolean;
      disabled?: boolean;
    }
  | { type: "separator" };

interface ContextMenuState {
  menu: { x: number; y: number; items: MenuItem[] } | null;
  open: (x: number, y: number, items: MenuItem[]) => void;
  close: () => void;
}

export const useContextMenuStore = create<ContextMenuState>((set) => ({
  menu: null,
  open: (x, y, items) => set({ menu: { x, y, items } }),
  close: () => set({ menu: null }),
}));
