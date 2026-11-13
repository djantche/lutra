import type { MouseEvent } from "react";
import { useContextMenuStore, type MenuItem } from "../state/contextMenu";

/**
 * Returns a `onContextMenu` handler that opens the app's own menu instead of the webview's.
 * The keyboard's menu key also lands here, with no pointer position: the menu then opens on the element.
 */
export function useContextMenu(getItems: () => MenuItem[]) {
  const open = useContextMenuStore((state) => state.open);

  return (event: MouseEvent<HTMLElement>) => {
    event.preventDefault();
    event.stopPropagation();

    let { clientX: x, clientY: y } = event;
    if (x === 0 && y === 0) {
      const rect = event.currentTarget.getBoundingClientRect();
      x = rect.left + rect.width / 2;
      y = rect.top + rect.height / 2;
    }

    const items = getItems();
    if (items.length > 0) open(x, y, items);
  };
}
