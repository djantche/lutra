import { useCallback, useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from "react";

interface VirtualGridProps<T> {
  items: T[];
  getKey: (item: T) => string;
  renderItem: (item: T, index: number) => ReactNode;
  /** Narrowest a column may be, in pixels; 0 for a single column (a list). */
  minColumnWidth: number;
  columnGap: number;
  rowGap: number;
  /** Rows drawn beyond the visible ones, so keyboard and controller moves find their target. */
  overscan?: number;
  className?: string;
}

/** The element whose scrolling moves `node`: the closest ancestor that scrolls, else the page. */
function scrollParent(node: HTMLElement | null): HTMLElement | null {
  for (let current = node?.parentElement ?? null; current; current = current.parentElement) {
    const { overflowY } = getComputedStyle(current);
    if (overflowY === "auto" || overflowY === "scroll") return current;
  }
  return null;
}

/**
 * A grid that only mounts the rows on screen. A library of thousands of games
 * would otherwise mount thousands of cards, each loading its artwork, which WebKitGTK cannot
 * draw. Every item must have the same height; it is measured on the first one.
 */
export function VirtualGrid<T>({
  items,
  getKey,
  renderItem,
  minColumnWidth,
  columnGap,
  rowGap,
  overscan = 2,
  className,
}: VirtualGridProps<T>) {
  const root = useRef<HTMLDivElement>(null);
  const first = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [itemHeight, setItemHeight] = useState(0);
  const [range, setRange] = useState({ start: 0, end: 0 });

  const columns =
    minColumnWidth <= 0 ? 1 : Math.max(1, Math.floor((width + columnGap) / (minColumnWidth + columnGap)));
  const rows = Math.ceil(items.length / columns);
  // Until the first item is measured, assume a poster card.
  const rowHeight = (itemHeight || ((width - (columns - 1) * columnGap) / columns) * 1.5 + 40) + rowGap;

  const update = useCallback(() => {
    const node = root.current;
    if (!node || rowHeight <= rowGap) return;
    const scroller = scrollParent(node);
    const viewTop = scroller ? scroller.getBoundingClientRect().top : 0;
    const viewHeight = scroller ? scroller.clientHeight : window.innerHeight;
    // Where the visible window falls inside the grid.
    const top = viewTop - node.getBoundingClientRect().top;
    const start = Math.max(0, Math.floor(top / rowHeight) - overscan);
    const end = Math.min(rows, Math.ceil((top + viewHeight) / rowHeight) + overscan);
    setRange((current) => (current.start === start && current.end === end ? current : { start, end }));
  }, [rowHeight, rowGap, rows, overscan]);

  useLayoutEffect(() => {
    const node = root.current;
    if (!node) return;
    const scroller = scrollParent(node);
    const observer = new ResizeObserver(() => {
      setWidth(node.clientWidth);
      update();
    });
    observer.observe(node);
    const target: HTMLElement | Window = scroller ?? window;
    target.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update);
    update();
    return () => {
      observer.disconnect();
      target.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, [update]);

  // The real height of an item, once one is on screen.
  useLayoutEffect(() => {
    const height = first.current?.offsetHeight ?? 0;
    if (height > 0 && height !== itemHeight) setItemHeight(height);
  });

  const visible = items.slice(range.start * columns, range.end * columns);
  const style = {
    position: "relative",
    height: rows > 0 ? rows * rowHeight - rowGap : 0,
    // Its rows are positioned, so it has no content of its own: in a flex column it would
    // otherwise be squeezed to nothing.
    flexShrink: 0,
  } as CSSProperties;
  const inner = {
    position: "absolute",
    top: range.start * rowHeight,
    left: 0,
    right: 0,
    display: "grid",
    gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))`,
    columnGap,
    rowGap,
  } as CSSProperties;

  return (
    <div ref={root} className={className} style={style}>
      <div style={inner}>
        {visible.map((item, offset) => (
          <div key={getKey(item)} ref={offset === 0 ? first : undefined}>
            {renderItem(item, range.start * columns + offset)}
          </div>
        ))}
      </div>
    </div>
  );
}
