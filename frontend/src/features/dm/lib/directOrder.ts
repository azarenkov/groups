export function sortDirects<T extends { name: string; lastAt?: string }>(items: T[]): T[] {
  const at = (d: T) => (d.lastAt ? Date.parse(d.lastAt) : Number.NaN);
  return [...items].sort((a, b) => {
    const ta = at(a);
    const tb = at(b);
    const hasA = Number.isFinite(ta);
    const hasB = Number.isFinite(tb);
    if (hasA && hasB && ta !== tb) return tb - ta;
    if (hasA !== hasB) return hasA ? -1 : 1;
    return a.name.localeCompare(b.name, undefined, { sensitivity: "base" });
  });
}

const DAY_MS = 24 * 60 * 60 * 1000;

const startOfDay = (d: Date) => new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();

export function formatListTime(iso: string | undefined, now: Date, locale?: string): string {
  if (!iso) return "";
  const at = new Date(iso);
  if (Number.isNaN(at.getTime())) return "";
  const days = Math.round((startOfDay(now) - startOfDay(at)) / DAY_MS);
  if (days <= 0) return at.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" });
  if (days < 7) return at.toLocaleDateString(locale, { weekday: "short" });
  if (at.getFullYear() === now.getFullYear()) {
    return at.toLocaleDateString(locale, { day: "numeric", month: "short" });
  }
  return at.toLocaleDateString(locale, { day: "2-digit", month: "2-digit", year: "2-digit" });
}

export function msUntilNextDay(now: Date): number {
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate() + 1);
  return Math.max(1000, next.getTime() - now.getTime() + 1000);
}
