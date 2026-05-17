import { writable } from "svelte/store";
import type { ColumnConfig } from "../types";
import { DEFAULT_COLUMNS } from "../types";

const STORAGE_KEY = "kathaloq-columns";

function loadColumns(): ColumnConfig[] {
  if (typeof window === "undefined") return DEFAULT_COLUMNS;
  const stored = localStorage.getItem(STORAGE_KEY);
  if (!stored) return DEFAULT_COLUMNS;
  try {
    const parsed = JSON.parse(stored) as ColumnConfig[];
    return DEFAULT_COLUMNS.map((def) => {
      const saved = parsed.find((c) => c.id === def.id);
      return saved ? { ...def, visible: saved.visible } : def;
    });
  } catch {
    return DEFAULT_COLUMNS;
  }
}

export const columns = writable<ColumnConfig[]>(loadColumns());

columns.subscribe((value) => {
  if (typeof window === "undefined") return;
  localStorage.setItem(STORAGE_KEY, JSON.stringify(value));
});

export function toggleColumn(id: string) {
  columns.update((cols) =>
    cols.map((c) => (c.id === id && c.id !== "name" ? { ...c, visible: !c.visible } : c))
  );
}
