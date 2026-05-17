import { writable } from "svelte/store";
import type { Theme } from "../types";

const STORAGE_KEY = "kathaloq-theme";

function getInitialTheme(): Theme {
  if (typeof window === "undefined") return "obsidian";
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && ["obsidian", "ember", "slate", "terminal"].includes(stored)) {
    return stored as Theme;
  }
  return "obsidian";
}

export const theme = writable<Theme>(getInitialTheme());

theme.subscribe((value) => {
  if (typeof window === "undefined") return;
  document.documentElement.setAttribute("data-theme", value);
  localStorage.setItem(STORAGE_KEY, value);
});
