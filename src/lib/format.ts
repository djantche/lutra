export function formatPlaytime(minutes: number): string {
  if (minutes < 60) return `${minutes} min`;
  const hours = minutes / 60;
  return `${hours >= 100 ? Math.round(hours) : Math.round(hours * 10) / 10} h`;
}

export function formatLastPlayed(iso: string | null, language: string, never: string): string {
  if (!iso) return never;
  const days = Math.floor((Date.now() - new Date(iso).getTime()) / 86_400_000);
  const rtf = new Intl.RelativeTimeFormat(language, { numeric: "auto" });
  const ago = (count: number, unit: Intl.RelativeTimeFormatUnit) =>
    rtf.format(count === 0 ? 0 : -count, unit);
  if (days < 30) return ago(days, "day");
  if (days < 365) return ago(Math.floor(days / 30), "month");
  return ago(Math.floor(days / 365), "year");
}

/** A size on disk, in the units people read: "2.4 GB", "830 MB". */
export function formatBytes(bytes: number, language: string): string {
  const units = ["byte", "kilobyte", "megabyte", "gigabyte", "terabyte"] as const;
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  return new Intl.NumberFormat(language, {
    style: "unit",
    unit: units[unit],
    unitDisplay: "short",
    maximumFractionDigits: value < 10 && unit > 0 ? 1 : 0,
  }).format(value);
}
