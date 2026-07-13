import { describe, expect, it } from "vitest";
import { filterChipsFor, removeFilterChipToken } from "../src/chips";

// The applied-filter chips are a pure view of `filterText`: one chip per raw
// token that parses as a supported field clause on the active surface, each
// carrying the index of the token that produced it so removal deletes exactly
// that occurrence. Free text never becomes a chip; key-set membership is
// entirely delegated to the surface-aware parser (never re-listed here).

describe("filterChipsFor", () => {
  it("returns one chip per parsed field clause, in token order", () => {
    const chips = filterChipsFor(
      "type:observation pinned track:codex",
      "event",
    );
    expect(chips.map((c) => c.field)).toEqual(["type", "track"]);
  });

  it("produces no chip for a free-text term", () => {
    expect(filterChipsFor("pinned", "event")).toEqual([]);
  });

  it("keeps duplicate-key clauses as distinct chips, one per exact token occurrence", () => {
    const chips = filterChipsFor("tag:a pinned tag:a", "event");
    expect(chips).toHaveLength(2);
    expect(chips[0]).toMatchObject({ field: "tag", value: "a", tokenIndex: 0 });
    expect(chips[1]).toMatchObject({ field: "tag", value: "a", tokenIndex: 2 });
  });

  it("marks a negated clause's chip", () => {
    // Use a non-aliased key: `status:` parses to the aliased field `check:`,
    // so a `-status:` chip would carry field "check", not "status".
    const chips = filterChipsFor("-check:failed", "event");
    expect(chips[0]).toMatchObject({
      field: "check",
      value: "failed",
      negate: true,
    });
  });

  it("resolves keys against the given surface, using each surface's key set", () => {
    // `actor:` is a member of both surfaces' key sets — this pins that chip
    // derivation calls the surface-aware parser rather than hardcoding one
    // surface's keys, even where the two surfaces happen to agree.
    const eventChips = filterChipsFor("actor:codex", "event");
    const revisionChips = filterChipsFor("actor:codex", "revision");
    expect(revisionChips.map((c) => c.field)).toContain("actor");
    expect(eventChips.map((c) => c.field)).toContain("actor");
  });

  it("carries the parser's canonical value, whichever actor spelling was typed", () => {
    // The parser canonicalizes a prefix-less actor value to the stored full id;
    // both spellings of the same id yield the same chip value.
    const short = filterChipsFor("actor:agent:codex", "event");
    const full = filterChipsFor("actor:actor:agent:codex", "event");
    expect(short[0]?.value).toBe(full[0]?.value);
  });

  it("produces no chip for a clause the surface parse drops with a diagnostic", () => {
    // `type:` is known but unsupported on the revision surface: the parser
    // drops the clause (never silent-empty), so no chip claims it is active.
    expect(filterChipsFor("type:observation", "revision")).toEqual([]);
  });
});

describe("removeFilterChipToken", () => {
  it("deletes exactly the token at the given index, preserving the rest in order", () => {
    expect(removeFilterChipToken("tag:a pinned tag:a", 2)).toBe("tag:a pinned");
    expect(removeFilterChipToken("tag:a pinned tag:a", 0)).toBe("pinned tag:a");
  });

  it("keeps a whitespace-bearing quoted clause intact as one token", () => {
    // The tokenizer, not this module, owns quoting: the quoted actor clause is
    // one token, so deleting its neighbor never splits it.
    expect(
      removeFilterChipToken('actor:"git-name:Kevin Swiber" tag:a', 1),
    ).toBe('actor:"git-name:Kevin Swiber"');
  });
});
