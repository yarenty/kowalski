/** Penguin avatar assets (`ui/src/assets/pinguins/<id>.png`). */

/** Source artwork dimensions (all assets in `assets/pinguins/*.png`). */
export const PENGUIN_AVATAR_SOURCE_PX = 256;

/** Display pixel size as a fraction of the 256px source (e.g. `0.25` → 64px). */
export function penguinDisplayPx(fraction: number): number {
  return Math.round(PENGUIN_AVATAR_SOURCE_PX * fraction);
}

/** Named display presets — UI never shows full 256px; scale down per surface. */
export const PENGUIN_DISPLAY = {
  /** Chat / federation feed lines (~12.5% → 32px) */
  inline: penguinDisplayPx(0.125),
  /** Pipeline cards (~18.75% → 48px) */
  card: penguinDisplayPx(0.1875),
  /** Editor header preview (~25% → 64px) */
  editor: penguinDisplayPx(0.25),
  /** Picker thumbnails (~15.6% → 40px) */
  picker: penguinDisplayPx(0.15625),
} as const;

export type PenguinDisplayVariant = keyof typeof PENGUIN_DISPLAY;

const modules = import.meta.glob<string>("../assets/pinguins/*.png", {
  eager: true,
  import: "default",
});

export const PENGUIN_AVATAR_URLS: Record<string, string> = {};
for (const path of Object.keys(modules)) {
  const id = path.replace(/.*\/([^/]+)\.png$/, "$1");
  PENGUIN_AVATAR_URLS[id] = modules[path];
}

export const PENGUIN_AVATAR_IDS = Object.keys(PENGUIN_AVATAR_URLS).sort((a, b) => {
  if (a === "default") return -1;
  if (b === "default") return 1;
  return a.localeCompare(b);
});

/** Resolve avatar id to bundled image URL; unknown ids fall back to `default`. */
export function penguinAvatarUrl(id: string | null | undefined): string {
  if (!id?.trim()) return PENGUIN_AVATAR_URLS.default ?? "";
  return PENGUIN_AVATAR_URLS[id] ?? PENGUIN_AVATAR_URLS.default ?? "";
}

/** Client-side mirror of `kowalski_core::infer_penguin_avatar` for previews before server round-trip. */
export function inferPenguinAvatarId(kind: string, name: string): string {
  const n = name.toLowerCase().replace(/-/g, "_");
  const patterns: [string, string][] = [
    ["mock_builder", "mock_builder"],
    ["mock", "mock_builder"],
    ["todo_generator", "todo_generator"],
    ["todo_list", "todo_generator"],
    ["todo", "todo_generator"],
    ["structure", "structure"],
    ["investigate", "investigate"],
    ["scaffold", "scaffold"],
    ["research", "researcher"],
    ["explorer", "explorer"],
    ["security", "security"],
    ["advisor", "advisor"],
    ["translator", "translator"],
    ["coordinator", "coordinator"],
    ["director", "director"],
    ["compile", "compile"],
    ["thinker", "thinker"],
    ["ingest", "ingest"],
    ["deliver", "deliver"],
    ["lint", "lint"],
    ["ask", "ask"],
  ];
  for (const [pat, avatar] of patterns) {
    if (n.includes(pat)) return avatar;
  }
  const k = kind.toLowerCase();
  if (k === "ingest") return "ingest";
  if (k === "deliver" || k === "final") return "deliver";
  if (k === "ask") return "ask";
  if (k === "lint") return "lint";
  if (k === "compile") return "compile";
  if (k === "investigate") return "investigate";
  if (k === "structure") return "structure";
  if (k === "scaffold") return "scaffold";
  if (k === "process" || k === "step") return "process";
  return "default";
}

export function resolvePenguinAvatar(
  avatar: string | null | undefined,
  kind: string,
  name: string,
): string {
  if (avatar?.trim()) return avatar.trim();
  return inferPenguinAvatarId(kind, name);
}

/** Labels for the avatar picker (id → display). */
export function penguinAvatarLabel(id: string): string {
  return id.replace(/_/g, " ");
}
