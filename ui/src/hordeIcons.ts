/**
 * Horde categories and icons: the single owner of how a horde is drawn (colour square + icon).
 *
 * The category list mirrors `HORDE_CATEGORIES` in `kowalski/src/horde.rs`; keep them in step.
 * Colours are theme tokens (`--cat-*` in `styles/theme.css`, light and dark); icons are 24px
 * stroke paths drawn in white on top of the category colour.
 */

export type HordeCategory = "spreadsheets" | "web" | "documents" | "code" | "other";

export type CategoryInfo = {
  id: HordeCategory;
  /** Chip / filter label. */
  label: string;
  /** CSS custom property holding the square's colour. */
  colorVar: string;
  /** Icon drawn when the horde names none. */
  defaultIcon: string;
};

export const HORDE_CATEGORIES: CategoryInfo[] = [
  { id: "spreadsheets", label: "Spreadsheets", colorVar: "--cat-spreadsheets", defaultIcon: "table" },
  { id: "web", label: "Web & news", colorVar: "--cat-web", defaultIcon: "globe" },
  { id: "documents", label: "Documents", colorVar: "--cat-documents", defaultIcon: "file" },
  { id: "code", label: "Code", colorVar: "--cat-code", defaultIcon: "code" },
  { id: "other", label: "Other", colorVar: "--cat-other", defaultIcon: "spark" },
];

const BY_ID = new Map(HORDE_CATEGORIES.map((c) => [c.id, c]));

/** 24×24 stroke icons (stroke-width ~1.8, round caps). `spark` is the fallback. */
export const HORDE_ICON_PATHS: Record<string, string> = {
  table: "M4 5h16v14H4zM4 10h16M4 14.5h16M10 5v14",
  sunrise: "M12 3v3M4.9 8.9l1.4 1.4M19.1 8.9l-1.4 1.4M2.5 17h19M7 17a5 5 0 0 1 10 0M6 20.5h12",
  inbox: "M4 13l2.4-7h11.2L20 13v6H4zM4 13h4.5l1 2h5l1-2H20",
  link: "M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1 1M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1-1",
  book: "M5 5.5A2.5 2.5 0 0 1 7.5 3H19v14.5H7.5A2.5 2.5 0 0 0 5 20zM5 20a1.5 1.5 0 0 0 1.5 1H19M9 7.5h6",
  code: "M8 8l-4 4 4 4M16 8l4 4-4 4M13.5 5l-3 14",
  blocks: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM16.5 13v7M13 16.5h7",
  file: "M6 3h8l4 4v14H6zM14 3v4h4M9 12h6M9 16h6",
  globe:
    "M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3.5 12h17M12 3c2.4 2.5 3.6 5.5 3.6 9s-1.2 6.5-3.6 9M12 3c-2.4 2.5-3.6 5.5-3.6 9s1.2 6.5 3.6 9",
  receipt: "M6 3h12v18l-2-1.4-2 1.4-2-1.4-2 1.4-2-1.4L6 21zM9 8h6M9 12h6M9 16h3",
  tag: "M3.5 12.5V4.5a1 1 0 0 1 1-1h8l8 8-9 9zM8 8h.01",
  chart: "M4 4v16h16M8 16v-4M12 16V8M16 16v-6",
  calendar: "M4 6h16v14H4zM4 10.5h16M8.5 3.5v4M15.5 3.5v4",
  search: "M10.5 4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13zM20 20l-4.8-4.8",
  spark: "M12 3l1.9 5.1L19 10l-5.1 1.9L12 17l-1.9-5.1L5 10l5.1-1.9zM18.5 15.5l.7 1.8 1.8.7-1.8.7-.7 1.8-.7-1.8-1.8-.7 1.8-.7z",
};

/** The category a horde belongs to; unknown or missing values read as `other`. */
export function categoryOf(category?: string | null): CategoryInfo {
  return BY_ID.get((category ?? "").trim().toLowerCase() as HordeCategory) ?? BY_ID.get("other")!;
}

/** Path data for a horde: its own icon when known, else its category's, else `spark`. */
export function iconPath(icon?: string | null, category?: string | null): string {
  const own = (icon ?? "").trim().toLowerCase();
  if (own && HORDE_ICON_PATHS[own]) return HORDE_ICON_PATHS[own];
  return HORDE_ICON_PATHS[categoryOf(category).defaultIcon] ?? HORDE_ICON_PATHS.spark;
}
