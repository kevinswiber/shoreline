import { describe, expect, it } from "vitest";
import type {
  Annotation,
  DiffArtifact,
  DiffCtx,
  DiffFile,
} from "../../src/diff/render";
import * as diffRender from "../../src/diff/render";
import {
  classifyLowSignal,
  fileFactCount,
  fileForFact,
  filePathLabel,
  fileRowCount,
  matchDiffFiles,
  rangeTouchesCapturedRows,
  renderAnnotation,
  renderDiff,
  renderDiffFactVicinity,
  renderDiffFileBody,
  renderDiffFileHeader,
  renderDiffNavSummary,
  unanchoredReason,
} from "../../src/diff/render";
import snapshotJson from "../fixtures/snapshot.json";

function parse(html: string): Document {
  return new DOMParser().parseFromString(html, "text/html");
}

const artifact = snapshotJson as unknown as DiffArtifact;
const libFile = (artifact.snapshot?.files ?? [])[0] as DiffFile;

// A range observation anchored to src/lib.rs:2 on the new side. In the fixture
// new_line 2 is the "    42" added row, so this fact anchors to a captured row.
const anchoredObs: Annotation = {
  kind: "observation",
  id: "obs:sha256:anchored",
  title: "Observed change",
  track: "agent:codex",
  body: "looks good",
  tags: ["needs-tests"],
  target: { kind: "range", filePath: "src/lib.rs", startLine: 2, endLine: 2 },
};

// A revision-level assessment belongs to Decision context, not an anchor error.
const unanchoredAssessment: Annotation = {
  kind: "assessment",
  id: "assess:sha256:broad",
  title: "assessment: accepted",
  track: "agent:codex",
  body: "",
  tags: [],
  target: { kind: "revision" },
};

const decisionValidation: Annotation = {
  kind: "validation",
  id: "validation:sha256:decision",
  title: "cargo test",
  track: "agent:codex",
  body: "tests passed",
  status: "passed",
  trigger: "manual",
  command: "cargo test",
  continuity: "current",
  writer: { actorId: "actor:agent:pointbreak-example-author" },
  target: { kind: "revision" },
};

const danglingFact: Annotation = {
  ...anchoredObs,
  id: "obs:sha256:dangling",
  title: "Missing file",
  target: { kind: "file", filePath: "src/missing.rs" },
};

function largeFile(rows: number): DiffFile {
  return {
    status: "modified",
    new_path: "src/big.rs",
    old_path: "src/big.rs",
    hunks: [
      {
        header: "@@ -1,1 +1,1 @@",
        rows: Array.from({ length: rows }, (_, i) => ({
          kind: "context",
          old_line: i + 1,
          new_line: i + 1,
          text: `line ${i + 1}`,
        })),
      },
    ],
  };
}

describe("filePathLabel", () => {
  it("shows both sides for a rename, else the single path", () => {
    expect(filePathLabel({ old_path: "a.rs", new_path: "b.rs" })).toBe(
      "a.rs → b.rs",
    );
    expect(filePathLabel({ old_path: "a.rs", new_path: "a.rs" })).toBe("a.rs");
    expect(filePathLabel({ new_path: "only.rs" })).toBe("only.rs");
    expect(filePathLabel({ old_path: "gone.rs" })).toBe("gone.rs");
    expect(filePathLabel({})).toBe("(unknown path)");
  });
});

describe("fileRowCount", () => {
  it("sums the rows across every hunk", () => {
    expect(fileRowCount(libFile)).toBe(9);
    expect(fileRowCount({})).toBe(0);
    expect(fileRowCount(largeFile(501))).toBe(501);
  });
});

describe("classifyLowSignal", () => {
  it("names binary and mode-only files", () => {
    expect(classifyLowSignal({ is_binary: true })).toBe("binary");
    expect(classifyLowSignal({ is_mode_only: true })).toBe("mode change only");
  });

  it("names a pure rename, with the similarity when present", () => {
    expect(
      classifyLowSignal({
        status: "renamed",
        old_path: "a.rs",
        new_path: "b.rs",
        hunks: [],
        similarity: 95,
      }),
    ).toBe("rename 95%");
    expect(
      classifyLowSignal({
        old_path: "a.rs",
        new_path: "b.rs",
        hunks: [],
      }),
    ).toBe("rename");
  });

  it("flags a file over the large-file row budget", () => {
    expect(classifyLowSignal(largeFile(501))).toBe("large file");
    expect(classifyLowSignal(largeFile(500))).toBeNull();
  });

  it("returns null for a normal content-bearing file", () => {
    expect(classifyLowSignal(libFile)).toBeNull();
  });
});

describe("fileFactCount", () => {
  it("counts anchored facts whose target file is either side of the file", () => {
    expect(fileFactCount(libFile, [anchoredObs])).toBe(1);
    expect(fileFactCount(libFile, [unanchoredAssessment])).toBe(0);
    expect(
      fileFactCount({ old_path: "a.rs", new_path: "b.rs" }, [
        { ...anchoredObs, target: { filePath: "a.rs" } },
      ]),
    ).toBe(1);
  });
});

describe("fileForFact", () => {
  it("finds the file matching either path, else null", () => {
    const files = artifact.snapshot?.files ?? [];
    expect(fileForFact(files, "src/lib.rs")).toBe(libFile);
    expect(fileForFact(files, "nope.rs")).toBeNull();
  });
});

describe("rangeTouchesCapturedRows", () => {
  it("is true when a captured row falls in the fact's line span", () => {
    expect(rangeTouchesCapturedRows(anchoredObs, libFile)).toBe(true);
  });

  it("is false when the span is outside every captured row", () => {
    expect(
      rangeTouchesCapturedRows(
        { ...anchoredObs, target: { kind: "range", startLine: 99 } },
        libFile,
      ),
    ).toBe(false);
  });

  it("treats a missing file or non-range target as touching (or not)", () => {
    expect(rangeTouchesCapturedRows(anchoredObs, null)).toBe(false);
    expect(
      rangeTouchesCapturedRows(
        { ...anchoredObs, target: { kind: "file" } },
        libFile,
      ),
    ).toBe(true);
  });
});

describe("unanchoredReason", () => {
  const filePaths = new Set(["src/lib.rs"]);

  it("labels a file/range target that omits its file path", () => {
    expect(
      unanchoredReason(
        { ...anchoredObs, target: { kind: "range", startLine: 2 } },
        filePaths,
      ),
    ).toBe("target missing file path");
  });

  it("does not invent an anchor-failure reason for Decision context", () => {
    expect(unanchoredReason(unanchoredAssessment, filePaths)).toBe(
      "not a file or range target",
    );
  });

  it("labels a range whose file is captured but line is outside the rows", () => {
    expect(
      unanchoredReason(
        {
          ...anchoredObs,
          kind: "observation",
          target: { kind: "range", filePath: "src/lib.rs" },
        },
        filePaths,
      ),
    ).toBe("line outside captured rows");
  });

  it("labels a file missing from the snapshot", () => {
    expect(
      unanchoredReason(
        {
          ...anchoredObs,
          kind: "observation",
          target: { kind: "file", filePath: "gone.rs" },
        },
        filePaths,
      ),
    ).toBe("file missing from snapshot");
  });
});

describe("renderAnnotation", () => {
  it("renders a kinded, tracked annotation with its body and tags", () => {
    const doc = parse(renderAnnotation(anchoredObs, false));
    const anno = doc.querySelector(".anno");
    expect(anno?.classList.contains("anno-observation")).toBe(true);
    expect(anno?.getAttribute("data-anno")).toBe("obs:sha256:anchored");
    expect(doc.querySelector(".anno-kind-observation")?.textContent).toBe(
      "observation",
    );
    expect(doc.querySelector(".anno-track")?.textContent).toBe("agent:codex");
    expect(doc.querySelector(".anno-title")?.textContent).toContain(
      "Observed change",
    );
    expect(doc.querySelector(".badge")?.textContent).toBe("needs-tests");
    expect(doc.querySelector(".anno-body")?.textContent).toContain(
      "looks good",
    );
  });

  it("includes a location only when asked and the target has a file", () => {
    expect(
      parse(renderAnnotation(anchoredObs, true)).querySelector(".anno-loc")
        ?.textContent,
    ).toBe("src/lib.rs:2-2 (new)");
    expect(
      parse(renderAnnotation(anchoredObs, false)).querySelector(".anno-loc"),
    ).toBeNull();
    expect(
      parse(renderAnnotation(unanchoredAssessment, true)).querySelector(
        ".anno-loc",
      ),
    ).toBeNull();
  });

  it("renders a markdown body when the content type selects it", () => {
    const doc = parse(
      renderAnnotation(
        {
          ...anchoredObs,
          body: "# Heading",
          bodyContentType: "text/markdown",
        },
        false,
      ),
    );
    expect(doc.querySelector(".markdown-body h1")?.textContent).toBe("Heading");
  });

  it("reuses exact actor, status, and nested-response fragments", () => {
    const opener = "actor:agent:pointbreak-example-author";
    const responder = "actor:agent:pointbreak-example-reviewer";
    const doc = parse(
      renderAnnotation(
        {
          kind: "input-request",
          id: "input-request:sha256:decision",
          title: "Ship this change?",
          track: "agent:review",
          body: "Please decide",
          status: "responded",
          mode: "operative",
          reasonCode: "manual_decision_required",
          writer: { actorId: opener },
          responses: [
            {
              id: "input-request-response:sha256:decision",
              outcome: "approved",
              reason: "evidence is sufficient",
              createdAt: "2026-07-17T08:00:00Z",
              writer: { actorId: responder },
            },
          ],
          target: { kind: "revision" },
        },
        true,
      ),
    );
    expect(doc.querySelector(".fact-status")?.textContent).toBe("responded");
    expect(
      doc.querySelector('.actor-attribution [data-ref-kind="actor"]')
        ?.textContent,
    ).toBe(opener);
    expect(doc.querySelector(".fact-response .outcome")?.textContent).toBe(
      "approved",
    );
    expect(
      doc.querySelector('.fact-response [data-ref-kind="actor"]')?.textContent,
    ).toBe(responder);
    expect(doc.querySelector(".fact-response")?.textContent).toContain(
      "evidence is sufficient",
    );
  });
});

describe("renderDiffFileHeader", () => {
  it("exposes the disclosure state and the eager fact-count badge", () => {
    const header = parse(
      renderDiffFileHeader(libFile, [anchoredObs], null, true),
    ).querySelector("header.dfile-head");
    expect(header?.getAttribute("role")).toBe("button");
    expect(header?.getAttribute("aria-expanded")).toBe("true");
    expect(header?.querySelector(".dstatus")?.textContent).toBe("modified");
    expect(header?.querySelector(".dpath")?.textContent).toBe("src/lib.rs");
    expect(header?.querySelector(".dfile-notes")?.textContent).toBe("1 note");
  });

  it("surfaces the low-signal reason and drops the badge with no facts", () => {
    const header = parse(
      renderDiffFileHeader(
        { is_binary: true, status: "modified", new_path: "logo.png" },
        [],
        "binary",
        false,
      ),
    ).querySelector("header.dfile-head");
    expect(header?.getAttribute("aria-expanded")).toBe("false");
    expect(header?.querySelector(".dfile-summary")?.textContent).toBe("binary");
    expect(header?.querySelector(".dfile-notes")).toBeNull();
  });
});

describe("renderDiffFileBody", () => {
  it("anchors a range fact to its captured row via the side:line map", () => {
    const doc = parse(renderDiffFileBody(libFile, [anchoredObs]));
    const noted = doc.querySelector(".drow-noted");
    expect(noted?.getAttribute("data-anno")).toBe("obs:sha256:anchored");
    // new_line 2 is the "    42" added row the fact anchors to.
    expect(noted?.querySelector(".dtext")?.textContent).toBe("    42");
    expect(noted?.classList.contains("drow-added")).toBe(true);
    // The annotation renders inline, once, after its row.
    expect(doc.querySelectorAll(".anno[data-anno]")).toHaveLength(1);
    expect(doc.querySelector(".dhunk")?.textContent).toBe("@@ -1,7 +1,7 @@");
  });

  it("renders a no-content note for an empty content-bearing file", () => {
    const doc = parse(
      renderDiffFileBody({ status: "added", new_path: "empty.rs" }, []),
    );
    expect(doc.querySelector(".drow-meta")?.textContent).toContain(
      "(no captured content)",
    );
  });
});

describe("renderDiffFactVicinity", () => {
  it("summarizes facts first with a hydrate-all affordance", () => {
    const doc = parse(renderDiffFactVicinity(libFile, [anchoredObs]));
    const vicinity = doc.querySelector(".diff-fact-vicinity");
    expect(vicinity?.getAttribute("data-fact-vicinity")).toBe("true");
    const btn = doc.querySelector("button[data-render-diff-file]");
    expect(btn?.textContent).toBe("Render all rows");
    expect(doc.querySelectorAll(".anno[data-anno]")).toHaveLength(1);
  });
});

describe("renderDiffNavSummary", () => {
  it("renders the file/fact/Decision-context/unanchored counts", () => {
    const doc = parse(
      renderDiffNavSummary({
        fileCount: 3,
        factCount: 7,
        decisionContextCount: 4,
        unanchoredCount: 2,
      }),
    );
    const summary = doc.querySelector(".diff-nav-summary");
    expect(summary?.getAttribute("aria-label")).toBe("diff summary");
    const bolds = Array.from(
      summary?.querySelectorAll("b") ?? [],
      (b) => b.textContent,
    );
    expect(bolds).toEqual(["3", "7", "4", "2"]);
  });
});

describe("renderDiff", () => {
  it("renders anchored, Decision context, and genuine unanchored facts separately", () => {
    const { html, ctx } = renderDiff("obj:sha256:lib", artifact, [
      anchoredObs,
      unanchoredAssessment,
      decisionValidation,
      danglingFact,
    ]);
    expect(ctx.snapshotId).toBe("obj:sha256:lib");
    expect(ctx.files).toBe(artifact.snapshot?.files);
    expect(ctx.anchored).toEqual([anchoredObs]);
    expect(ctx.decisionContext).toEqual([
      unanchoredAssessment,
      decisionValidation,
    ]);
    expect(ctx.unanchored).toEqual([danglingFact]);
    expect(ctx.filePaths.has("src/lib.rs")).toBe(true);

    const doc = parse(html);
    // The summary names the fact breakdown and only the true unanchored count.
    expect(doc.querySelector(".anno-summary")?.textContent).toContain(
      "not anchored to a diff line",
    );
    const decision = doc.querySelector(".diff-decision-context");
    expect(decision?.getAttribute("aria-label")).toBe("Decision context");
    expect(decision?.classList.contains("hidden")).toBe(false);
    expect(decision?.querySelector("h2")?.textContent).toContain(
      "Decision context",
    );
    expect(decision?.querySelector(".anno-assessment")).not.toBeNull();
    expect(decision?.querySelector(".anno-validation")).not.toBeNull();
    const unanchored = doc.querySelector(".diff-unanchored-facts");
    expect(unanchored?.querySelector("h2")?.textContent).toContain(
      "Unanchored facts",
    );
    expect(unanchored?.querySelector(".anno-observation")).not.toBeNull();
    expect(
      Array.from(doc.body.children).indexOf(decision as Element),
    ).toBeLessThan(
      Array.from(doc.body.children).indexOf(
        doc.querySelector(".dfile") as Element,
      ),
    );
  });

  it("exports the fixed three-way partition as a pure classifier", () => {
    const partition = (
      diffRender as unknown as {
        partitionAnnotations: (
          files: DiffFile[],
          annotations: Annotation[],
        ) => {
          anchored: Annotation[];
          decisionContext: Annotation[];
          unanchored: Annotation[];
        };
      }
    ).partitionAnnotations(artifact.snapshot?.files ?? [], [
      anchoredObs,
      unanchoredAssessment,
      decisionValidation,
      danglingFact,
    ]);
    expect(partition).toEqual({
      anchored: [anchoredObs],
      decisionContext: [unanchoredAssessment, decisionValidation],
      unanchored: [danglingFact],
    });
  });

  it("renders each file as an accordion section with the disclosure on the header", () => {
    const { html } = renderDiff("obj:sha256:lib", artifact, [anchoredObs]);
    const doc = parse(html);
    const section = doc.querySelector("section.dfile");
    expect(section?.getAttribute("data-dfile")).toBe("0");
    expect(section?.getAttribute("data-expanded")).toBe("true");
    // The section wrapper does not own the disclosure aria state.
    expect(section?.hasAttribute("aria-expanded")).toBe(false);
    expect(section?.querySelector("header.dfile-head")).not.toBeNull();
    const body = section?.querySelector(".dfile-body");
    expect(body?.getAttribute("data-dfile-body")).toBe("0");
    expect(body?.getAttribute("data-rendered")).toBe("1");
  });

  it("marks a low-signal file and renders it collapsed", () => {
    const binaryArtifact: DiffArtifact = {
      snapshot: {
        files: [{ status: "modified", new_path: "logo.png", is_binary: true }],
      },
    };
    const { html } = renderDiff("obj:sha256:bin", binaryArtifact, []);
    const section = parse(html).querySelector("section.dfile");
    expect(section?.getAttribute("data-lowsignal")).toBe("binary");
    expect(section?.getAttribute("data-expanded")).toBe("false");
    expect(section?.querySelector(".dfile-body")?.innerHTML).toBe("");
  });

  it("renders an annotated large file as a fact vicinity, not full rows", () => {
    const big = largeFile(600);
    const fact: Annotation = {
      ...anchoredObs,
      target: { kind: "file", filePath: "src/big.rs" },
    };
    const { html } = renderDiff(
      "obj:sha256:big",
      { snapshot: { files: [big] } },
      [fact],
    );
    const body = parse(html).querySelector(".dfile-body");
    expect(body?.getAttribute("data-fact-vicinity")).toBe("true");
    expect(body?.querySelector(".diff-fact-vicinity")).not.toBeNull();
    expect(body?.querySelector(".dhunk")).toBeNull();
  });

  it("notes an empty snapshot", () => {
    const { html } = renderDiff(
      "obj:sha256:empty",
      { snapshot: { files: [] } },
      [],
    );
    expect(parse(html).querySelector(".empty")?.textContent).toContain(
      "No files captured in this snapshot.",
    );
  });
});

describe("renderDiffFileBody syntax tokens", () => {
  it("renders syntax spans inside .dtext when tokens present", () => {
    const file: DiffFile = {
      status: "modified",
      old_path: "a.rs",
      new_path: "a.rs",
      hunks: [
        {
          header: "@@ -1 +1 @@",
          rows: [
            {
              kind: "added",
              old_line: null,
              new_line: 1,
              text: "let x",
              tokens: [{ start: 0, end: 3, kind: "keyword" }],
            },
          ],
        },
      ],
    };
    const html = renderDiffFileBody(file, []);
    expect(html).toContain('<span class="tok tok-keyword">let</span>');
  });

  it("renders plain escaped text when tokens absent (unchanged from today)", () => {
    const file: DiffFile = {
      status: "modified",
      old_path: "a.rs",
      new_path: "a.rs",
      hunks: [
        {
          header: "@@ -1 +1 @@",
          rows: [{ kind: "context", old_line: 1, new_line: 1, text: "a < b" }],
        },
      ],
    };
    const html = renderDiffFileBody(file, []);
    expect(html).toContain("a &lt; b");
    expect(html).not.toContain("tok-");
  });
});

describe("renderDiffFileBody emphasis", () => {
  it("renders emphasis from the wire row", () => {
    const file: DiffFile = {
      status: "modified",
      old_path: "m.rs",
      new_path: "m.rs",
      hunks: [
        {
          header: "@@ -1 +1 @@",
          rows: [
            {
              kind: "added",
              old_line: null,
              new_line: 1,
              text: "let x",
              emphasis: [{ start: 4, end: 5 }],
            },
          ],
        },
      ],
    };
    const html = renderDiffFileBody(file, []);
    expect(html).toContain('<span class="emph">x</span>');
  });
});

describe("matchDiffFiles", () => {
  function ctxWith(
    files: DiffFile[],
    anchored: Annotation[] = [],
    unanchored: Annotation[] = [],
  ): DiffCtx {
    const filePaths = new Set<string>();
    for (const f of files) {
      if (f.new_path) filePaths.add(f.new_path);
      if (f.old_path) filePaths.add(f.old_path);
    }
    return {
      snapshotId: "obj:sha256:test",
      files,
      anchored,
      decisionContext: [],
      unanchored,
      filePaths,
    };
  }

  const fileAdded: DiffFile = { status: "added", new_path: "src/added.rs" };
  const fileDeleted: DiffFile = {
    status: "deleted",
    old_path: "src/deleted.rs",
  };
  const fileRenamed: DiffFile = {
    status: "renamed",
    old_path: "src/old.rs",
    new_path: "src/new.rs",
  };

  it("path: matches a substring of the file path label", () => {
    const ctx = ctxWith([libFile, fileAdded]);
    const { files } = matchDiffFiles(ctx, "path:lib");
    expect(files).toHaveLength(1);
    expect(files[0]).toBe(libFile);
  });

  it("free text with no colon matches the path substring too", () => {
    const ctx = ctxWith([libFile, fileAdded]);
    expect(matchDiffFiles(ctx, "lib").files).toEqual([libFile]);
  });

  it("change: matches exactly on the real DiffFile.status enum (added/deleted/modified/renamed/copied)", () => {
    const ctx = ctxWith([fileAdded, fileDeleted, fileRenamed, libFile]);
    expect(matchDiffFiles(ctx, "change:added").files).toEqual([fileAdded]);
    expect(matchDiffFiles(ctx, "change:deleted").files).toEqual([fileDeleted]);
    expect(matchDiffFiles(ctx, "change:renamed").files).toEqual([fileRenamed]);
    expect(matchDiffFiles(ctx, "change:modified").files).toEqual([libFile]);
  });

  it("change: with an unrecognized value drops the clause with a diagnostic, never silent-empty", () => {
    const ctx = ctxWith([libFile]);
    const { files, diagnostics } = matchDiffFiles(ctx, "change:bogus");
    expect(files).toEqual([libFile]); // the clause is not applied — not treated as always-false
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0]).toMatchObject({
      code: "unsupported-value",
      key: "change",
    });
  });

  it("has:facts matches files carrying at least one anchored fact", () => {
    // anchoredObs targets src/lib.rs (module-level fixture, defined above).
    const ctx = ctxWith([libFile, fileAdded], [anchoredObs]);
    expect(matchDiffFiles(ctx, "has:facts").files).toEqual([libFile]);
  });

  it("has: with an unrecognized value drops the clause with a diagnostic", () => {
    const ctx = ctxWith([libFile], [anchoredObs]);
    const { files, diagnostics } = matchDiffFiles(ctx, "has:bogus");
    expect(files).toEqual([libFile]);
    expect(diagnostics).toEqual([
      expect.objectContaining({ code: "unsupported-value", key: "has" }),
    ]);
  });

  it("is:unanchored matches a file targeted by a fact that fell outside its captured rows", () => {
    // A range fact whose file IS captured but whose line span is outside every
    // captured row: renderDiff's own partition classifies it unanchored (see
    // rangeTouchesCapturedRows) — this pins is:unanchored to that same partition
    // rather than reinventing the anchored/unanchored split.
    const outsideRowsFact: Annotation = {
      ...anchoredObs,
      id: "obs:sha256:outside",
      target: {
        kind: "range",
        filePath: "src/lib.rs",
        startLine: 999,
        endLine: 999,
      },
    };
    const { ctx } = renderDiff("obj:sha256:lib", artifact, [outsideRowsFact]);
    expect(ctx.unanchored).toEqual([outsideRowsFact]); // pin: renderDiff already classifies this unanchored
    expect(matchDiffFiles(ctx, "is:unanchored").files).toEqual([libFile]);
  });

  it("is:unanchored excludes files when the unanchored fact targets no captured file", () => {
    // unanchoredAssessment is revision-level (target.kind === "revision") — it
    // maps to no file, so no file should match is:unanchored from it.
    const { ctx } = renderDiff("obj:sha256:lib", artifact, [
      unanchoredAssessment,
    ]);
    expect(matchDiffFiles(ctx, "is:unanchored").files).toEqual([]);
  });

  it("is: with an unrecognized value drops the clause with a diagnostic", () => {
    const ctx = ctxWith([libFile]);
    const { files, diagnostics } = matchDiffFiles(ctx, "is:bogus");
    expect(files).toEqual([libFile]);
    expect(diagnostics).toEqual([
      expect.objectContaining({ code: "unsupported-value", key: "is" }),
    ]);
  });

  it("status: is never valid at file scope — a pointed diagnostic, never silent-empty", () => {
    const ctx = ctxWith([libFile]);
    const { files, diagnostics } = matchDiffFiles(ctx, "status:modified");
    expect(files).toEqual([libFile]); // the clause is dropped, not applied
    expect(diagnostics).toHaveLength(1);
    expect(diagnostics[0].key).toBe("status");
    expect(diagnostics[0].code).toBe("unsupported-qualifier");
    expect(diagnostics[0].message).toContain("change:");
  });

  it("an unrecognized key falls through as free text, unstripped of its colon", () => {
    const ctx = ctxWith([libFile, fileAdded]);
    const { files, diagnostics } = matchDiffFiles(ctx, "author:nobody");
    expect(diagnostics).toEqual([]);
    expect(files).toEqual([]); // "author:nobody" is not a substring of either path label
  });

  it("an empty query matches every file with no diagnostics", () => {
    const ctx = ctxWith([libFile, fileAdded]);
    const { files, diagnostics } = matchDiffFiles(ctx, "");
    expect(files).toEqual([libFile, fileAdded]);
    expect(diagnostics).toEqual([]);
  });
});
