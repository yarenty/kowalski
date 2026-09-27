/** How a run's API status reads in lists (Runs page, horde page, picker, home). */
export type RunTone = "ok" | "red" | "running" | "muted";

export function runStatusLabel(status: string): { label: string; tone: RunTone } {
  switch (status) {
    case "completed":
      return { label: "Done", tone: "ok" };
    case "awaiting_input":
      return { label: "Needs you", tone: "red" };
    case "failed":
      return { label: "Failed", tone: "red" };
    case "running":
    case "pending":
      return { label: "Running", tone: "running" };
    case "cancelled":
      return { label: "Cancelled", tone: "muted" };
    default:
      return { label: status || "Unknown", tone: "muted" };
  }
}

function toDate(iso?: string | null): Date | null {
  if (!iso) return null;
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? null : d;
}

/** "just now", "5 min ago", "3 h ago", then "Mon 14:05" within a week, else a date. */
export function relativeTime(iso?: string | null): string {
  const d = toDate(iso);
  if (!d) return "";
  const diff = Date.now() - d.getTime();
  if (diff < 0) return clock(d);
  const min = Math.floor(diff / 60000);
  if (min < 1) return "just now";
  if (min < 60) return `${min} min ago`;
  const hr = Math.floor(min / 60);
  if (hr < 24) return `${hr} h ago`;
  if (hr < 24 * 7) return d.toLocaleDateString([], { weekday: "short" }) + " " + clock(d);
  return d.toLocaleDateString([], { day: "numeric", month: "short", year: "numeric" });
}

/** A future time: "today 07:00", "tomorrow 07:00", or a short date and time. */
export function upcomingTime(iso?: string | null): string {
  const d = toDate(iso);
  if (!d) return "";
  const days = dayDiff(d);
  if (days === 0) return `today ${clock(d)}`;
  if (days === -1) return `tomorrow ${clock(d)}`;
  return `${d.toLocaleDateString([], { weekday: "short", day: "numeric", month: "short" })} ${clock(d)}`;
}

export function clock(d: Date): string {
  return d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

export function clockOf(iso?: string | null): string {
  const d = toDate(iso);
  return d ? clock(d) : "";
}

/** Whole calendar days from `d` to today (0 = today, 1 = yesterday, -1 = tomorrow). */
function dayDiff(d: Date): number {
  const a = new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
  const now = new Date();
  const b = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  return Math.round((b - a) / 86400000);
}

/** Day group header for a run list: "Today", "Yesterday", then a date. */
export function dayLabel(iso?: string | null): string {
  const d = toDate(iso);
  if (!d) return "Earlier";
  const days = dayDiff(d);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  return d.toLocaleDateString([], { weekday: "long", day: "numeric", month: "long" });
}

/** "4 s", "2 min 05 s", "1 h 12 min". */
export function elapsed(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s} s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} min ${String(s % 60).padStart(2, "0")} s`;
  return `${Math.floor(m / 60)} h ${m % 60} min`;
}
