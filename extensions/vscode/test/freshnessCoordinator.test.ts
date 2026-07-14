import { readdirSync, readFileSync, statSync } from "node:fs";
import { basename, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import type { InspectFreshnessDoc } from "../src/cli";
import {
  type FreshnessAttention,
  type FreshnessConnection,
  FreshnessCoordinator,
  type FreshnessPanels,
} from "../src/freshnessCoordinator";
import type { InspectSession } from "../src/inspectChild";
import { InspectClientError } from "../src/inspectClient";

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

it("polls only a visible authenticated session and stable markers are no-ops", async () => {
  const harness = coordinatorHarness();

  harness.attention.setVisible(true);
  expect(vi.getTimerCount()).toBe(0);
  expect(harness.connection.activeSession).toHaveBeenCalledOnce();

  harness.connection.setSession(
    session("a", [marker(1, "one"), marker(1, "one")]),
  );
  await settled();
  expect(harness.clients.a.freshness).toHaveBeenCalledOnce();
  expect(harness.attention.refreshTarget).toHaveBeenCalledWith(
    "a",
    expect.any(AbortSignal),
  );
  expect(vi.getTimerCount()).toBe(1);

  harness.attention.refreshTarget.mockClear();
  await vi.advanceTimersByTimeAsync(25);
  expect(harness.clients.a.freshness).toHaveBeenCalledTimes(2);
  expect(harness.attention.refreshTarget).not.toHaveBeenCalled();
  expect(harness.panels.reloadActive).not.toHaveBeenCalled();

  harness.attention.setVisible(false);
  expect(vi.getTimerCount()).toBe(0);
  await vi.advanceTimersByTimeAsync(50);
  expect(harness.clients.a.freshness).toHaveBeenCalledTimes(2);

  harness.coordinator.dispose();
});

it("refreshes typed target documents and the active diff on either marker moving", async () => {
  const harness = coordinatorHarness();
  harness.panels.setVisible(true);
  harness.connection.setSession(
    session("a", [marker(1, "one"), marker(2, "one"), marker(2, "two")]),
  );
  await settled();
  harness.attention.refreshTarget.mockClear();

  await vi.advanceTimersByTimeAsync(25);
  expect(harness.attention.refreshTarget).toHaveBeenLastCalledWith(
    "a",
    expect.any(AbortSignal),
  );
  expect(harness.panels.reloadActive).toHaveBeenLastCalledWith(
    "a",
    expect.any(AbortSignal),
  );

  await vi.advanceTimersByTimeAsync(25);
  expect(harness.attention.refreshTarget).toHaveBeenCalledTimes(2);
  expect(harness.panels.reloadActive).toHaveBeenCalledTimes(2);

  harness.coordinator.dispose();
});

it("compares event count only while either commit graph stamp is absent", async () => {
  const harness = coordinatorHarness();
  harness.attention.setVisible(true);
  harness.connection.setSession(
    session("a", [marker(4), marker(4, "one"), marker(4, "two"), marker(5)]),
  );
  await settled();
  harness.attention.refreshTarget.mockClear();

  await vi.advanceTimersByTimeAsync(25);
  expect(harness.attention.refreshTarget).not.toHaveBeenCalled();
  await vi.advanceTimersByTimeAsync(25);
  expect(harness.attention.refreshTarget).toHaveBeenCalledOnce();
  await vi.advanceTimersByTimeAsync(25);
  expect(harness.attention.refreshTarget).toHaveBeenCalledTimes(2);

  harness.coordinator.dispose();
});

it("invalidates a stale poll across target switches, stops, hiding, and disposal", async () => {
  const harness = coordinatorHarness();
  const stale = deferred<InspectFreshnessDoc>();
  harness.attention.setVisible(true);
  harness.connection.setSession(session("a", [stale.promise]));
  await settled();

  harness.connection.setSession(session("b", [marker(9, "b")]));
  await settled();
  harness.attention.refreshTarget.mockClear();
  stale.resolve(marker(99, "stale"));
  await settled();
  expect(harness.attention.refreshTarget).not.toHaveBeenCalled();
  expect(harness.panels.reloadActive).not.toHaveBeenCalled();

  harness.connection.setSession(undefined);
  expect(vi.getTimerCount()).toBe(0);
  harness.connection.setSession(session("b", [marker(10, "b")]));
  await settled();
  expect(vi.getTimerCount()).toBe(1);
  harness.coordinator.dispose();
  expect(vi.getTimerCount()).toBe(0);
});

it("keeps freshness failures distinct without refreshing cached surfaces", async () => {
  const harness = coordinatorHarness();
  harness.attention.setVisible(true);
  harness.connection.setSession(
    session("a", [
      new InspectClientError("unauthorized"),
      new InspectClientError("unreachable"),
      new InspectClientError("protocol"),
    ]),
  );
  await settled();
  await vi.advanceTimersByTimeAsync(50);

  expect(harness.reportError.mock.calls.map(([error]) => error.kind)).toEqual([
    "unauthorized",
    "unreachable",
    "protocol",
  ]);
  expect(harness.attention.refreshTarget).toHaveBeenCalledOnce();
  expect(harness.panels.reloadActive).not.toHaveBeenCalled();
  expect(harness.connection.activeSession).toHaveBeenCalled();

  harness.coordinator.dispose();
});

it("is the sole interval and direct freshness owner in extension source", () => {
  const files = sourceFiles(fileURLToPath(new URL("../src", import.meta.url)));
  const intervalOwners = files.filter((file) =>
    readFileSync(file, "utf8").includes("setInterval("),
  );
  const freshnessCallers = files.filter((file) =>
    readFileSync(file, "utf8").includes(".freshness()"),
  );
  const source = files.map((file) => readFileSync(file, "utf8")).join("\n");

  expect(intervalOwners.map((file) => basename(file))).toEqual([
    "freshnessCoordinator.ts",
  ]);
  expect(freshnessCallers.map((file) => basename(file))).toEqual([
    "freshnessCoordinator.ts",
  ]);
  expect(source).not.toContain("POLL_INTERVAL_MS");
});

it("keeps manual and post-write refresh immediate without warming other targets", async () => {
  const harness = coordinatorHarness();

  await harness.coordinator.refreshAll();
  expect(harness.attention.refresh).toHaveBeenCalledOnce();
  expect(harness.panels.reloadActive).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);

  await harness.coordinator.refreshAfterWrite();
  expect(harness.attention.refresh).toHaveBeenCalledTimes(2);
  expect(harness.panels.reloadActive).toHaveBeenCalledOnce();
  expect(vi.getTimerCount()).toBe(0);

  harness.coordinator.dispose();
});

function coordinatorHarness() {
  const attention = fakeAttention();
  const panels = fakePanels();
  const connection = fakeConnection();
  const reportError = vi.fn<(error: InspectClientError) => void>();
  const coordinator = new FreshnessCoordinator(connection, attention, panels, {
    intervalMs: 25,
    reportError,
  });
  return {
    attention,
    clients,
    connection,
    coordinator,
    panels,
    reportError,
  };
}

const clients: Record<string, { freshness: ReturnType<typeof vi.fn> }> = {};

function session(
  targetKey: string,
  values: Array<
    InspectFreshnessDoc | Promise<InspectFreshnessDoc> | InspectClientError
  >,
): InspectSession {
  const freshness = vi.fn();
  for (const value of values) {
    if (value instanceof InspectClientError) {
      freshness.mockRejectedValueOnce(value);
    } else {
      freshness.mockImplementationOnce(() => Promise.resolve(value));
    }
  }
  clients[targetKey] = { freshness };
  return { targetKey, client: { freshness } as never };
}

function marker(
  eventCount: number,
  commitGraphStamp?: string,
): InspectFreshnessDoc {
  return {
    schema: "pointbreak.inspect-freshness",
    version: 1,
    eventCount,
    ...(commitGraphStamp === undefined ? {} : { commitGraphStamp }),
  };
}

function fakeAttention(): FreshnessAttention & {
  setVisible(visible: boolean): void;
  refresh: ReturnType<typeof vi.fn>;
  refreshTarget: ReturnType<typeof vi.fn>;
} {
  const visibility = eventSource<boolean>();
  let visible = false;
  return {
    isVisible: () => visible,
    onDidChangeVisibility: visibility.event,
    refresh: vi.fn(async () => undefined),
    refreshTarget: vi.fn(async () => undefined),
    setVisible(value) {
      visible = value;
      visibility.fire(value);
    },
  };
}

function fakePanels(): FreshnessPanels & {
  setVisible(visible: boolean): void;
  reloadActive: ReturnType<typeof vi.fn>;
} {
  const visibility = eventSource<boolean>();
  let visible = false;
  return {
    isVisible: () => visible,
    onDidChangeVisibility: visibility.event,
    reloadActive: vi.fn(async () => undefined),
    setVisible(value) {
      visible = value;
      visibility.fire(value);
    },
  };
}

function fakeConnection(): FreshnessConnection & {
  setSession(session: InspectSession | undefined): void;
  activeSession: ReturnType<typeof vi.fn>;
} {
  const changes = eventSource<{ targetKey: string } | undefined>();
  let current: InspectSession | undefined;
  const activeSession = vi.fn(() => current);
  return {
    activeSession,
    onDidChangeSession: changes.event,
    setSession(value) {
      current = value;
      changes.fire(value ? { targetKey: value.targetKey } : undefined);
    },
  };
}

function eventSource<T>() {
  const listeners = new Set<(value: T) => unknown>();
  return {
    event(listener: (value: T) => unknown) {
      listeners.add(listener);
      return { dispose: () => listeners.delete(listener) };
    },
    fire(value: T) {
      for (const listener of listeners) listener(value);
    },
  };
}

function deferred<T>() {
  let resolve!: (value: T | PromiseLike<T>) => void;
  const promise = new Promise<T>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

async function settled(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

function sourceFiles(directory: string): string[] {
  return readdirSync(directory)
    .flatMap((entry) => {
      const path = join(directory, entry);
      return statSync(path).isDirectory() ? sourceFiles(path) : [path];
    })
    .filter((path) => path.endsWith(".ts"));
}
