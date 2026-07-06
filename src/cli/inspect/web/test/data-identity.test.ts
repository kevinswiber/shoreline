import { afterEach, beforeEach, expect, it } from "vitest";
import identityJson from "./fixtures/identity.json";
import { mountInspectorDom, resetDom } from "./support/dom";
import { installFetchMock, uninstallFetchMock } from "./support/fetch";

// `data.ts`'s `loadIdentity` fetches the static `/api/identity` document once and
// commits it to the store (issue #391). The store, like the other data modules, is a
// module singleton — reset and re-import it before each test.
type Store = typeof import("../src/store");
type Data = typeof import("../src/data");
let store: Store;
let data: Data;

beforeEach(async () => {
  const vitest = await import("vitest");
  vitest.vi.resetModules();
  store = await import("../src/store");
  data = await import("../src/data");
  mountInspectorDom();
  installFetchMock();
});

afterEach(() => {
  uninstallFetchMock();
  resetDom();
});

it("identity starts null", () => {
  expect(store.getState().identity).toBeNull();
});

it("loadIdentity commits the fetched identity document", async () => {
  await data.loadIdentity();
  expect(store.getState().identity).toEqual(identityJson);
});
