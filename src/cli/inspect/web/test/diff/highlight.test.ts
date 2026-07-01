import { describe, expect, it } from "vitest";
import { highlightRowText } from "../../src/diff/highlight";

describe("highlightRowText", () => {
  it("empty/absent tokens is byte-identical to escapeHtml", () => {
    expect(highlightRowText("a < b & c", [])).toBe("a &lt; b &amp; c");
    expect(highlightRowText("a < b & c")).toBe("a &lt; b &amp; c");
  });

  it("wraps non-plain spans, escaping each segment, leaving gaps plain", () => {
    expect(
      highlightRowText("let x", [{ start: 0, end: 3, kind: "keyword" }]),
    ).toBe('<span class="tok tok-keyword">let</span> x');
  });

  it("escapes inside a span", () => {
    expect(
      highlightRowText("a<b", [{ start: 0, end: 3, kind: "operator" }]),
    ).toBe('<span class="tok tok-operator">a&lt;b</span>');
  });

  it("slices by UTF-16 units (multibyte safe)", () => {
    // offsets are UTF-16 code units: "é " = 2 units
    expect(
      highlightRowText("é let", [{ start: 2, end: 5, kind: "keyword" }]),
    ).toBe('é <span class="tok tok-keyword">let</span>');
  });

  it("malformed/out-of-range spans fall back to escapeHtml(text)", () => {
    expect(
      highlightRowText("a<b", [{ start: 5, end: 9, kind: "keyword" }]),
    ).toBe("a&lt;b");
    expect(
      highlightRowText("a<b", [{ start: 2, end: 1, kind: "keyword" }]),
    ).toBe("a&lt;b");
  });

  it("emphasis-only wraps just the changed span", () => {
    expect(highlightRowText("let x", undefined, [{ start: 4, end: 5 }])).toBe(
      'let <span class="emph">x</span>',
    );
  });

  it("token + emphasis overlap merges classes on the shared segment", () => {
    // keyword [0,3), emphasis [0,2) → "le" carries both, "t" carries only the token
    expect(
      highlightRowText(
        "let x",
        [{ start: 0, end: 3, kind: "keyword" }],
        [{ start: 0, end: 2 }],
      ),
    ).toBe(
      '<span class="tok tok-keyword emph">le</span><span class="tok tok-keyword">t</span> x',
    );
  });

  it("invalid emphasis is dropped but valid tokens still render", () => {
    expect(
      highlightRowText(
        "let x",
        [{ start: 0, end: 3, kind: "keyword" }],
        [{ start: 1, end: 99 }],
      ),
    ).toBe('<span class="tok tok-keyword">let</span> x');
  });

  it("emphasis escapes the changed segment", () => {
    expect(highlightRowText("a<b", undefined, [{ start: 0, end: 3 }])).toBe(
      '<span class="emph">a&lt;b</span>',
    );
  });
});
