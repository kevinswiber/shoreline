import { afterEach, beforeEach, describe, expect, it } from "vitest";
import {
  applyDensity,
  applyPrefs,
  applySplit,
  applyTheme,
  initControls,
  preferredSplit,
  preferredTheme,
  setDensity,
  setThemeMode,
  watchColorScheme,
} from "../src/prefs";
import { mountInspectorDom, resetDom } from "./support/dom";

// The persisted storage keys (the reader-local preference contract; mirrors app.js).
const THEME_KEY = "shore-inspect-theme";
const DENSITY_KEY = "shore-inspect-density";
const SPLIT_KEY = "shore-inspect-split";

const realMatchMedia = window.matchMedia;

function fakeMediaQueryList(matches: boolean, media: string): MediaQueryList {
  return {
    matches,
    media,
    onchange: null,
    addEventListener: () => {},
    removeEventListener: () => {},
    addListener: () => {},
    removeListener: () => {},
    dispatchEvent: () => false,
  };
}

/** Make `prefers-color-scheme: light` resolve deterministically. */
function stubPrefersLight(prefersLight: boolean): void {
  window.matchMedia = (query: string) =>
    fakeMediaQueryList(prefersLight && query.includes("light"), query);
}

/** A matchMedia stub whose OS preference can flip live, firing registered `change` handlers. */
function stubControllableColorScheme(initialPrefersLight: boolean): {
  setPrefersLight(next: boolean): void;
} {
  let prefersLight = initialPrefersLight;
  const handlers: Array<(e: MediaQueryListEvent) => void> = [];
  window.matchMedia = (query: string): MediaQueryList => {
    const isLightQuery = query.includes("light");
    return {
      get matches() {
        return isLightQuery ? prefersLight : !prefersLight;
      },
      media: query,
      onchange: null,
      addEventListener: (
        _type: string,
        cb: EventListenerOrEventListenerObject,
      ) => {
        handlers.push(cb as (e: MediaQueryListEvent) => void);
      },
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    } as MediaQueryList;
  };
  return {
    setPrefersLight(next: boolean): void {
      prefersLight = next;
      for (const cb of handlers) cb({ matches: next } as MediaQueryListEvent);
    },
  };
}

beforeEach(() => {
  mountInspectorDom();
  localStorage.clear();
  stubPrefersLight(false);
});

afterEach(() => {
  resetDom();
  localStorage.clear();
  window.matchMedia = realMatchMedia;
});

describe("preferredTheme", () => {
  it("returns the stored theme when it is light or dark", () => {
    localStorage.setItem(THEME_KEY, "light");
    expect(preferredTheme()).toBe("light");
    localStorage.setItem(THEME_KEY, "dark");
    expect(preferredTheme()).toBe("dark");
  });

  it("falls back to the OS color-scheme preference when unset", () => {
    stubPrefersLight(true);
    expect(preferredTheme()).toBe("light");
    stubPrefersLight(false);
    expect(preferredTheme()).toBe("dark");
  });

  it("ignores a junk stored value and uses the OS preference", () => {
    localStorage.setItem(THEME_KEY, "neon");
    stubPrefersLight(true);
    expect(preferredTheme()).toBe("light");
  });
});

describe("applyTheme / setThemeMode", () => {
  it("applyTheme sets data-theme on the document root", () => {
    applyTheme("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
  });

  it("checks the explicit pinned theme in the View panel", () => {
    localStorage.setItem(THEME_KEY, "dark");
    applyTheme("dark");
    expect(
      document.querySelector<HTMLInputElement>("#theme-dark")?.checked,
    ).toBe(true);
    localStorage.setItem(THEME_KEY, "light");
    applyTheme("light");
    expect(
      document.querySelector<HTMLInputElement>("#theme-light")?.checked,
    ).toBe(true);
  });

  it("keeps system selected while its resolved theme changes", () => {
    applyTheme("dark");
    expect(
      document.querySelector<HTMLInputElement>("#theme-system")?.checked,
    ).toBe(true);
    applyTheme("light");
    expect(
      document.querySelector<HTMLInputElement>("#theme-system")?.checked,
    ).toBe(true);
  });

  it("sets an explicit theme and can restore system following", () => {
    stubPrefersLight(false);
    setThemeMode("light");
    expect(localStorage.getItem(THEME_KEY)).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    setThemeMode("dark");
    expect(localStorage.getItem(THEME_KEY)).toBe("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    setThemeMode("system");
    expect(localStorage.getItem(THEME_KEY)).toBe("system");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(
      document.querySelector<HTMLInputElement>("#theme-system")?.checked,
    ).toBe(true);
  });
});

describe("applyDensity / setDensity", () => {
  it("applyDensity toggles the compact class on the root", () => {
    applyDensity("compact");
    expect(document.documentElement.classList.contains("compact")).toBe(true);
    applyDensity("comfortable");
    expect(document.documentElement.classList.contains("compact")).toBe(false);
  });

  it("checks the applied density in the View panel", () => {
    applyDensity("compact");
    expect(
      document.querySelector<HTMLInputElement>("#density-compact")?.checked,
    ).toBe(true);
    applyDensity("comfortable");
    expect(
      document.querySelector<HTMLInputElement>("#density-comfortable")?.checked,
    ).toBe(true);
  });

  it("sets and persists an explicit density", () => {
    setDensity("compact");
    expect(document.documentElement.classList.contains("compact")).toBe(true);
    expect(localStorage.getItem(DENSITY_KEY)).toBe("compact");
    setDensity("comfortable");
    expect(document.documentElement.classList.contains("compact")).toBe(false);
    expect(localStorage.getItem(DENSITY_KEY)).toBe("comfortable");
  });
});

describe("applyPrefs", () => {
  it("applies the stored theme and density (the before-first-paint step)", () => {
    localStorage.setItem(THEME_KEY, "light");
    localStorage.setItem(DENSITY_KEY, "compact");
    applyPrefs();
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(document.documentElement.classList.contains("compact")).toBe(true);
  });

  it("defaults density to comfortable when unset", () => {
    applyPrefs();
    expect(document.documentElement.classList.contains("compact")).toBe(false);
  });

  it("seeds the View-panel choices from stored prefs at first paint", () => {
    localStorage.setItem(THEME_KEY, "light");
    localStorage.setItem(DENSITY_KEY, "compact");
    applyPrefs();
    expect(
      document.querySelector<HTMLInputElement>("#theme-light")?.checked,
    ).toBe(true);
    expect(
      document.querySelector<HTMLInputElement>("#density-compact")?.checked,
    ).toBe(true);
  });
});

describe("preferredSplit / applySplit (the divider width pref)", () => {
  it("applyPrefs sets --split-master from the stored width", () => {
    localStorage.setItem(SPLIT_KEY, "62");
    applyPrefs();
    expect(
      document.documentElement.style.getPropertyValue("--split-master"),
    ).toBe("62%");
  });

  it("defaults to the 50/50 grid when the width pref is unset or out of range", () => {
    applyPrefs();
    expect(
      document.documentElement.style.getPropertyValue("--split-master"),
    ).toBe("");
    localStorage.setItem(SPLIT_KEY, "9000");
    expect(preferredSplit()).toBeNull();
    applyPrefs();
    expect(
      document.documentElement.style.getPropertyValue("--split-master"),
    ).toBe("");
  });

  it("applySplit persists and clamps; null clears the property and the key", () => {
    applySplit(62);
    expect(localStorage.getItem(SPLIT_KEY)).toBe("62");
    applySplit(99);
    expect(localStorage.getItem(SPLIT_KEY)).toBe("75");
    applySplit(null);
    expect(
      document.documentElement.style.getPropertyValue("--split-master"),
    ).toBe("");
    expect(localStorage.getItem(SPLIT_KEY)).toBeNull();
  });
});

describe("watchColorScheme", () => {
  it("re-applies the theme live when the OS preference flips and no theme is pinned", () => {
    const media = stubControllableColorScheme(false);
    applyPrefs();
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    watchColorScheme();
    media.setPrefersLight(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    expect(
      document.querySelector<HTMLInputElement>("#theme-system")?.checked,
    ).toBe(true);
    media.setPrefersLight(false);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
    expect(
      document.querySelector<HTMLInputElement>("#theme-system")?.checked,
    ).toBe(true);
  });

  it("ignores OS changes once the reader has pinned an explicit theme", () => {
    const media = stubControllableColorScheme(false);
    localStorage.setItem(THEME_KEY, "dark");
    applyPrefs();
    watchColorScheme();
    media.setPrefersLight(true);
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });
});

describe("initControls", () => {
  it("wires the View panel's explicit theme and density choices", () => {
    applyPrefs(); // system mode, OS dark ⇒ data-theme dark
    initControls();
    const light = document.querySelector<HTMLInputElement>("#theme-light");
    if (light) light.checked = true;
    light?.dispatchEvent(new Event("change", { bubbles: true }));
    expect(localStorage.getItem(THEME_KEY)).toBe("light");
    expect(document.documentElement.getAttribute("data-theme")).toBe("light");
    const compact =
      document.querySelector<HTMLInputElement>("#density-compact");
    if (compact) compact.checked = true;
    compact?.dispatchEvent(new Event("change", { bubbles: true }));
    expect(document.documentElement.classList.contains("compact")).toBe(true);
  });
});
