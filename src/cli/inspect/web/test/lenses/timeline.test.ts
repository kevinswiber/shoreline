import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HistoryDoc } from "../../src/store";
import type { HistoryEntry } from "../../src/types";
import historyJson from "../fixtures/history.json";
import { mountInspectorDom, resetDom } from "../support/dom";

// `lenses/timeline.ts` paints the event timeline into the master pane. It is
// state-reading + DOM-writing: seed the store, inject the timeline body the way
// `renderMaster` does at runtime (the static shell leaves `#master` empty), then
// assert the painted rows. Rows carry the `data-event-id` delegation dataset and
// no per-row listener — the `#master` delegate (a later PR) handles selection, so
// a row click here changes nothing. The store and the lens are module singletons
// sharing one `state`, so reset the registry and re-import both before each test.
type Store = typeof import("../../src/store");
type Timeline = typeof import("../../src/lenses/timeline");
let store: Store;
let timeline: Timeline;

beforeEach(async () => {
  vi.resetModules();
  store = await import("../../src/store");
  timeline = await import("../../src/lenses/timeline");
  mountInspectorDom();
  // renderMaster (a later PR) injects the timeline body inside #master; mirror it.
  const master = document.querySelector("#master");
  if (master) master.innerHTML = `<ol id="timeline" class="timeline"></ol>`;
  history.replaceState(null, "", "/");
});

afterEach(() => {
  resetDom();
});

function seedHistory(entries: HistoryEntry[]): void {
  store.commit({
    history: { entries, diagnostics: [] } as unknown as HistoryDoc,
  });
}

/** A minimal timeline entry with a canonical (non-retired) event type. */
function entry(
  eventId: string,
  over: Partial<HistoryEntry> = {},
): HistoryEntry {
  return {
    eventId,
    eventType: "review_observation_recorded",
    occurredAt: "unix-ms:1782699185391",
    ...over,
  };
}

function rowIds(): string[] {
  return Array.from(
    document.querySelectorAll<HTMLElement>("#timeline li.event"),
  )
    .map((li) => li.dataset.eventId ?? "")
    .filter(Boolean);
}

describe("renderTimeline", () => {
  it("paints one row per event with the data-event-id delegation dataset", () => {
    store.commit({ history: historyJson as unknown as HistoryDoc });
    // A real load enables every present type; mirror it so the full history paints
    // (the default toggles cover only the canonical TYPES set).
    const entries = (historyJson as unknown as HistoryDoc).entries;
    store.commit({ enabledTypes: new Set(entries.map((e) => e.eventType)) });
    timeline.renderTimeline();
    const rows = document.querySelectorAll<HTMLElement>("#timeline li.event");
    expect(rows.length).toBe(entries.length);
    for (const li of rows) expect(li.dataset.eventId).toBeTruthy();
  });

  it("renders newest-first by default and reverses for ascending order", () => {
    seedHistory([entry("e1"), entry("e2"), entry("e3")]);
    timeline.renderTimeline();
    expect(rowIds()).toEqual(["e3", "e2", "e1"]);

    store.commit({ order: "asc" });
    timeline.renderTimeline();
    expect(rowIds()).toEqual(["e1", "e2", "e3"]);
  });

  it("drops retired-lineage event types not present in the timeline type set", () => {
    seedHistory([
      entry("capture", { eventType: "work_object_proposed" }),
      entry("lineage", { eventType: "review_unit_lineage" }),
    ]);
    timeline.renderTimeline();
    const ids = rowIds();
    expect(ids).toContain("capture");
    expect(ids).not.toContain("lineage");
  });

  it("renders an advisory verification chip from the entry status", () => {
    seedHistory([entry("e1", { verificationStatus: "unsigned" })]);
    timeline.renderTimeline();
    const chip = document.querySelector("#timeline li.event .verify");
    expect(chip).not.toBeNull();
    expect(chip?.textContent).toContain("unsigned");
  });

  it("linkifies embedded reference ids into navigable chips, not plain text", () => {
    const ref =
      "rev:sha256:9a7626ca7cb2801721ed992402184460210477aadfd4f7228628b65ff11a6efd";
    seedHistory([entry("e1", { summary: { title: `supersedes ${ref}` } })]);
    timeline.renderTimeline();
    const chip = document.querySelector<HTMLElement>(
      "#timeline li.event [data-ref-kind]",
    );
    // The chip carries the ref-navigation dataset so the #master delegate's
    // `closest("[data-ref-kind]")` guard leaves it to the navigation delegate.
    expect(chip).not.toBeNull();
    expect(chip?.dataset.refKind).toBe("rev");
    expect(chip?.dataset.refId).toBe(ref);
  });

  it("attaches no per-row click listener — selection is left to the #master delegate", () => {
    seedHistory([entry("e1")]);
    timeline.renderTimeline();
    const row = document.querySelector<HTMLElement>("#timeline li.event");
    row?.dispatchEvent(new Event("click", { bubbles: true }));
    // No lens-attached listener selected the row; the route is untouched.
    expect(store.getState().selected).toEqual({ kind: null, id: null });
  });

  it("shows a muted empty-state row when no events match the filters", () => {
    seedHistory([entry("e1")]);
    store.commit({ enabledTypes: new Set<string>() });
    timeline.renderTimeline();
    expect(rowIds()).toEqual([]);
    expect(document.querySelector("#timeline")?.textContent).toContain(
      "no events match",
    );
  });
});
