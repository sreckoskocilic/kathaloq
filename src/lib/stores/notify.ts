import { writable } from "svelte/store";

export interface Notice {
  id: number;
  message: string;
}

let nextId = 0;

const MAX_NOTICES = 5;

export const notices = writable<Notice[]>([]);

export function notifyError(context: string, error: unknown): void {
  const detail = error instanceof Error ? error.message : String(error);
  const message = detail && detail !== "undefined" ? `${context}: ${detail}` : context;
  const id = ++nextId;
  notices.update((all) => [...all, { id, message }].slice(-MAX_NOTICES));
}

export function dismissNotice(id: number): void {
  notices.update((all) => all.filter((n) => n.id !== id));
}
