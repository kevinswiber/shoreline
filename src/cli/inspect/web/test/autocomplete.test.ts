import { describe, expect, it } from "vitest";
import { acceptSuggestion, suggestionsFor } from "../src/autocomplete";

const NO_DISTINCT = { track: [], actor: [], tag: [] };

describe("suggestionsFor — key suggestions", () => {
  it("suggests event-surface keys matching the trailing token's prefix", () => {
    const s = suggestionsFor("tra", "event", NO_DISTINCT);
    expect(s.map((x) => x.insertText)).toContain("track:");
  });

  it("suggests revision-surface keys that don't exist on the event surface", () => {
    const s = suggestionsFor("attent", "revision", NO_DISTINCT);
    expect(s.map((x) => x.insertText)).toContain("attention:");
  });

  it("returns nothing for an empty trailing token", () => {
    expect(suggestionsFor("type:observation ", "event", NO_DISTINCT)).toEqual(
      [],
    );
  });
});

describe("suggestionsFor — value suggestions", () => {
  it("offers type: values from the closed TYPES label set", () => {
    const s = suggestionsFor("type:obs", "event", NO_DISTINCT);
    expect(s.map((x) => x.insertText)).toContain("type:observation");
  });

  it("scopes type: values to the present type ids when provided", () => {
    // A store with no review_initialized events must not offer `type:init` —
    // the same present-types authority the facet menu renders from.
    const present = new Set(["review_observation_recorded"]);
    const vals = suggestionsFor("type:", "event", NO_DISTINCT, present).map(
      (x) => x.insertText,
    );
    expect(vals).toContain("type:observation");
    expect(vals).not.toContain("type:init");
  });

  it("offers an unknown present type id raw, since the grammar accepts wire ids", () => {
    const present = new Set(["review_observation_recorded", "custom_event"]);
    const vals = suggestionsFor("type:cust", "event", NO_DISTINCT, present).map(
      (x) => x.insertText,
    );
    expect(vals).toEqual(["type:custom_event"]);
  });

  it("offers track:/actor:/tag: values from distinctValues", () => {
    const distinct = {
      track: ["agent:codex"],
      actor: ["human:kevin"],
      tag: ["issue"],
    };
    expect(
      suggestionsFor("track:cod", "event", distinct).map((x) => x.insertText),
    ).toEqual(["track:agent:codex"]);
    expect(
      suggestionsFor("tag:iss", "event", distinct).map((x) => x.insertText),
    ).toEqual(["tag:issue"]);
  });

  it("offers actor: values in the short spelling and matches either typed spelling", () => {
    // distinctValues.actor carries FULL ids (`actor:agent:codex`); the parser
    // canonicalizes the short spelling back to the full id, and the UI (chips,
    // actor-ref clicks) mints the short form — suggestions do the same, and a
    // partially-typed value completes from either spelling.
    const distinct = { track: [], actor: ["actor:agent:codex"], tag: [] };
    expect(
      suggestionsFor("actor:age", "event", distinct).map((x) => x.insertText),
    ).toEqual(["actor:agent:codex"]);
    expect(
      suggestionsFor("actor:actor:age", "event", distinct).map(
        (x) => x.insertText,
      ),
    ).toEqual(["actor:agent:codex"]);
    expect(
      suggestionsFor("actor:age", "event", distinct).map((x) => x.label),
    ).toEqual(["actor:agent:codex"]);
  });

  it("quotes a whitespace-bearing value so the inserted clause survives tokenization", () => {
    // Fallback Git-name actor ids legally contain spaces; unquoted, the
    // inserted clause would split into a field clause plus stray free text.
    const distinct = {
      track: [],
      actor: ["actor:git-name:kevin swiber"],
      tag: [],
    };
    expect(
      suggestionsFor("actor:kev", "event", distinct).map((x) => x.insertText),
    ).toEqual(['actor:"git-name:kevin swiber"']);
    // A partially-typed quoted spelling still completes.
    expect(
      suggestionsFor('actor:"git', "event", distinct).map((x) => x.insertText),
    ).toEqual(['actor:"git-name:kevin swiber"']);
  });

  it("offers is: values from the closed per-surface set", () => {
    const eventVals = suggestionsFor("is:", "event", NO_DISTINCT).map(
      (x) => x.insertText,
    );
    const revisionVals = suggestionsFor("is:", "revision", NO_DISTINCT).map(
      (x) => x.insertText,
    );
    expect(eventVals).toContain("is:open");
    expect(eventVals).not.toContain("is:contested"); // event surface has no contested predicate
    expect(revisionVals).toContain("is:contested");
  });

  it("offers check:/assessment: values from their closed enums", () => {
    expect(
      suggestionsFor("check:fail", "event", NO_DISTINCT).map(
        (x) => x.insertText,
      ),
    ).toEqual(["check:failed"]);
    expect(
      suggestionsFor("assessment:accepted", "revision", NO_DISTINCT).map(
        (x) => x.insertText,
      ),
    ).toContain("assessment:accepted");
  });

  it("returns nothing for an unrecognized key", () => {
    expect(suggestionsFor("bogus:x", "event", NO_DISTINCT)).toEqual([]);
  });

  it("offers attention: values on the revision surface from the shared closed token set", () => {
    const revisionVals = suggestionsFor(
      "attention:open",
      "revision",
      NO_DISTINCT,
    ).map((x) => x.insertText);
    expect(revisionVals).toContain("attention:open-request");
    // `attention` is not an event-surface key — the outer key-set gate rejects
    // it before any value lookup runs.
    expect(suggestionsFor("attention:open", "event", NO_DISTINCT)).toEqual([]);
  });
});

describe("acceptSuggestion", () => {
  it("replaces the trailing token and appends a trailing space", () => {
    expect(acceptSuggestion("type:obs", "type:observation")).toBe(
      "type:observation ",
    );
    expect(acceptSuggestion("pinned track:cod", "track:agent:codex")).toBe(
      "pinned track:agent:codex ",
    );
  });
});
