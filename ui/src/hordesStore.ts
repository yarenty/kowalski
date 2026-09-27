import { ref } from "vue";
import { api, type HordeCatalogItem } from "./api";

/**
 * The horde catalogue, shared by the Hordes home, the horde page, the Runs page and the ⌘K
 * picker. The server hot-reloads horde definitions, so the list is re-read every 15 s while
 * any screen uses it.
 */
export const hordes = ref<HordeCatalogItem[]>([]);
export const hordesLoaded = ref(false);
export const hordesError = ref<string | null>(null);

let inflight: Promise<void> | null = null;
export function refreshHordes(): Promise<void> {
  if (inflight) return inflight;
  inflight = api
    .hordes()
    .then((res) => {
      hordes.value = res.hordes ?? [];
      hordesError.value = null;
    })
    .catch((e) => {
      hordesError.value = e instanceof Error ? e.message : String(e);
    })
    .finally(() => {
      hordesLoaded.value = true;
      inflight = null;
    });
  return inflight;
}

export function hordeById(id: string | null | undefined): HordeCatalogItem | null {
  if (!id) return null;
  return hordes.value.find((h) => h.id === id) ?? null;
}

let users = 0;
let timer: ReturnType<typeof setInterval> | null = null;
/** Start polling for a mounted screen; call the returned function on unmount. */
export function useHordePolling(): () => void {
  users += 1;
  void refreshHordes();
  if (!timer) timer = setInterval(() => void refreshHordes(), 15000);
  return () => {
    users -= 1;
    if (users <= 0 && timer) {
      clearInterval(timer);
      timer = null;
      users = 0;
    }
  };
}

/** First sentence of a description, for one-line tiles and picker rows. */
export function firstSentence(text?: string | null): string {
  const t = (text ?? "").replace(/\s+/g, " ").trim();
  const m = /^(.+?[.!?])(\s|$)/.exec(t);
  return m ? m[1] : t;
}
