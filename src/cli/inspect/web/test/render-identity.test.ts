import { afterEach, beforeEach, expect, it } from "vitest";
import type { IdentityDoc } from "../src/store";
import { mountInspectorDom, resetDom } from "./support/dom";
import { installFetchMock, uninstallFetchMock } from "./support/fetch";

// `renderIdentity` (inside the single `render()` subscriber) paints the top-bar
// repo/store identity and sets the browser tab `<title>` (issue #391). Module
// singletons (store, render), so reset and re-import before each test.
type Store = typeof import("../src/store");
type Render = typeof import("../src/render");
let store: Store;
let render: Render;

const CLONE: IdentityDoc = {
  repository: "shoreline",
  placement: { tier: "clone", label: "clone store" },
};

beforeEach(async () => {
  const vitest = await import("vitest");
  vitest.vi.resetModules();
  store = await import("../src/store");
  render = await import("../src/render");
  mountInspectorDom();
  installFetchMock();
});

afterEach(() => {
  uninstallFetchMock();
  resetDom();
  document.title = "shore inspector";
});

it("paints the repository and placement, and sets the tab title", () => {
  store.commit({ identity: CLONE });
  render.render();
  const el = document.querySelector("#store-identity");
  expect(el?.textContent).toContain("shoreline");
  expect(el?.textContent).toContain("clone store");
  expect(document.title).toBe("shoreline · shore inspector");
});

it("omits family and worktree when absent", () => {
  store.commit({ identity: CLONE });
  render.render();
  const el = document.querySelector("#store-identity");
  expect(el?.querySelector(".store-identity-family")).toBeNull();
  expect(el?.querySelector(".store-identity-worktree")).toBeNull();
});

it("shows the family chip under the user-level tier", () => {
  store.commit({
    identity: {
      repository: "shoreline",
      placement: { tier: "family", label: "family store" },
      family: { id: "acme-web" },
    },
  });
  render.render();
  expect(document.querySelector(".store-identity-family")?.textContent).toBe(
    "acme-web",
  );
});

it("shows the worktree chip when present and distinct", () => {
  store.commit({
    identity: {
      repository: "shoreline",
      worktree: "feat-foo",
      placement: { tier: "clone", label: "clone store" },
    },
  });
  render.render();
  expect(document.querySelector(".store-identity-worktree")?.textContent).toBe(
    "feat-foo",
  );
});

it("falls back to the default title when identity is null", () => {
  store.commit({ identity: null });
  render.render();
  expect(document.title).toBe("shore inspector");
  expect(document.querySelector("#store-identity")?.textContent).toBe("");
});
