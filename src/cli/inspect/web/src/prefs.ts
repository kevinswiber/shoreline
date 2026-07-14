// Reader-local display preferences (theme / density / split width). Ported from
// the served app.js prefs cluster. These are reader-local choices, persisted in
// localStorage and never encoded in the URL/hash — they are not shareable view
// state, so they live off the store, applied straight to the document root as
// app.js does. The split width is an integer master-pane percent (25–75) applied
// as the `--split-master` custom property the `.split` grid reads; unset means
// the 50/50 default.
//
// Unlike app.js, the apply-on-load is NOT a top-level import side-effect: `main`
// calls `applyPrefs()` explicitly before the first paint.

import { $ } from "./dom";

const THEME_KEY = "shore-inspect-theme";
const DENSITY_KEY = "shore-inspect-density";
const SPLIT_KEY = "shore-inspect-split";

const SPLIT_MIN = 25;
const SPLIT_MAX = 75;

// Strong references to the MediaQueryLists we've attached `change` listeners to.
// WebKit/Safari garbage-collects a MediaQueryList that is reachable only through
// its own listener, which silently stops the listener from ever firing; holding a
// reference here keeps the query alive (Chromium/Firefox retain it regardless).
const liveMediaQueries: MediaQueryList[] = [];

const densityListeners: Array<() => void> = [];

/** Register a callback to reconcile layout after a density change. */
export function registerDensityListener(listener: () => void): void {
  densityListeners.push(listener);
}

/** Notify every layout consumer that the density changed. */
export function notifyDensityListeners(): void {
  for (const listener of densityListeners) listener();
}

// The three explicit theme choices. `system` follows the OS color scheme live
// (see watchColorScheme); `light`/`dark` pin an explicit choice.
// Persisted under THEME_KEY — anything else (unset or junk) reads as `system`, so
// a fresh reader follows the OS.
type ThemeMode = "system" | "light" | "dark";

/** The reader's stored theme mode; unset or junk reads as `system` (follow the OS). */
function preferredThemeMode(): ThemeMode {
  const stored = localStorage.getItem(THEME_KEY);
  return stored === "light" || stored === "dark" ? stored : "system";
}

/** True when the reader has pinned light/dark, so the OS preference is ignored. */
function hasPinnedTheme(): boolean {
  return preferredThemeMode() !== "system";
}

/** The OS color-scheme preference, resolved to `light`/`dark`. */
function osTheme(): string {
  return window.matchMedia("(prefers-color-scheme: light)").matches
    ? "light"
    : "dark";
}

/** The effective theme: the pinned `light`/`dark`, else the OS preference. */
export function preferredTheme(): string {
  const mode = preferredThemeMode();
  return mode === "system" ? osTheme() : mode;
}

function syncChoice(name: string, value: string): void {
  for (const input of document.querySelectorAll<HTMLInputElement>(
    `input[name="${name}"]`,
  )) {
    input.checked = input.value === value;
  }
}

/**
 * Apply the resolved theme and keep the explicit choice in the View panel in
 * sync. System remains checked while its resolved light/dark value changes.
 */
export function applyTheme(theme: string): void {
  document.documentElement.setAttribute("data-theme", theme);
  syncChoice("theme-mode", preferredThemeMode());
}

/** Persist one explicit theme choice and apply its resolved color scheme. */
export function setThemeMode(mode: string): void {
  const next: ThemeMode = mode === "light" || mode === "dark" ? mode : "system";
  localStorage.setItem(THEME_KEY, next);
  applyTheme(preferredTheme());
}

/** The stored density, defaulting to `comfortable` when unset. */
function preferredDensity(): string {
  return localStorage.getItem(DENSITY_KEY) || "comfortable";
}

/** Apply a density and keep its View-panel choice synchronized. */
export function applyDensity(mode: string): void {
  const value = mode === "compact" ? "compact" : "comfortable";
  document.documentElement.classList.toggle("compact", value === "compact");
  syncChoice("density-mode", value);
}

/** Persist one explicit density choice and apply it. */
export function setDensity(mode: string): void {
  const next = mode === "compact" ? "compact" : "comfortable";
  localStorage.setItem(DENSITY_KEY, next);
  applyDensity(next);
}

/** The stored split width (integer master percent, 25–75), or null for the 50/50 default. */
export function preferredSplit(): number | null {
  const raw = localStorage.getItem(SPLIT_KEY);
  const n = raw === null ? Number.NaN : Number.parseInt(raw, 10);
  return Number.isInteger(n) && n >= SPLIT_MIN && n <= SPLIT_MAX ? n : null;
}

/**
 * Apply (and persist) a split width, clamped to the valid range so the stored
 * pref can never hold junk from our own writers; null clears back to the 50/50
 * default (property and key both removed). The divider controller is the only
 * post-paint caller — every width write goes through here.
 */
export function applySplit(pct: number | null): void {
  if (pct === null) {
    document.documentElement.style.removeProperty("--split-master");
    localStorage.removeItem(SPLIT_KEY);
    return;
  }
  const clamped = Math.round(Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, pct)));
  document.documentElement.style.setProperty("--split-master", `${clamped}%`);
  localStorage.setItem(SPLIT_KEY, String(clamped));
}

/**
 * Apply the persisted theme + density + split width. `main` calls this before
 * the first paint so the chosen theme is in place immediately (reproduces
 * app.js's top-level `applyTheme(preferredTheme())` / `applyDensity(...)`, as an
 * explicit call). A stored-but-invalid split key is left untouched and simply
 * not applied (the default grid wins).
 */
export function applyPrefs(): void {
  applyTheme(preferredTheme());
  applyDensity(preferredDensity());
  const split = preferredSplit();
  if (split !== null) applySplit(split);
}

/**
 * Follow live OS color-scheme changes while the reader hasn't pinned a theme.
 * `applyPrefs` reads the OS preference once before first paint; without this,
 * a later system light/dark switch only takes effect on the next page load.
 * A pinned light/dark choice (via the toggle) still wins over the OS.
 *
 * The query is retained in `liveMediaQueries` so Safari doesn't garbage-collect
 * it out from under its own listener (see the note on that binding).
 */
export function watchColorScheme(): void {
  const query = window.matchMedia("(prefers-color-scheme: light)");
  liveMediaQueries.push(query);
  query.addEventListener("change", () => {
    if (hasPinnedTheme()) return;
    applyTheme(preferredTheme());
  });
}

/** Wire the View panel's display choices and the OS color-scheme watcher. */
export function initControls(): void {
  $("#view-panel")?.addEventListener("change", (event) => {
    const input = event.target;
    if (!(input instanceof HTMLInputElement) || !input.checked) return;
    if (input.name === "theme-mode") setThemeMode(input.value);
    if (input.name === "density-mode") {
      setDensity(input.value);
      notifyDensityListeners();
    }
  });
  watchColorScheme();
}
