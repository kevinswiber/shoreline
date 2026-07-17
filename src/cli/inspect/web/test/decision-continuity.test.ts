import { describe, expect, it } from "vitest";
import type { RevisionPageDoc } from "../src/detail";
import {
  compositeAnnotations,
  renderAssociationAndLanding,
} from "../src/detail";
import { renderDecisionContext } from "../src/diff/render";
import { type Revision, revisionSearchIndex } from "../src/projection";
import { workLabelText } from "../src/refs";
import uncertaintyJson from "./fixtures/decision-continuity-uncertainty.json";
import canonicalJson from "./fixtures/review-example-decision.json";

const canonical = canonicalJson as unknown as RevisionPageDoc;
const uncertainty = uncertaintyJson as unknown as RevisionPageDoc;

describe("decision-continuity fixture contracts", () => {
  it("keeps the canonical no-summary loop attributed and identical across detail and routed diff", () => {
    const annotations = compositeAnnotations(canonical);
    const kinds = annotations.map((annotation) => annotation.kind);
    expect(kinds).toEqual([
      "observation",
      "observation",
      "observation",
      "input-request",
      "assessment",
      "assessment",
      "validation",
      "validation",
      "validation",
    ]);
    expect(canonical.revision?.summary).toBeUndefined();
    expect(workLabelText(canonical.revision?.targetDisplay)).toBe(
      "Fix null-user checkout",
    );
    expect(annotations[0].writer?.actorId).toBe(
      "actor:agent:pointbreak-example-author",
    );
    expect(annotations[3].writer?.actorId).toBe(
      "actor:agent:pointbreak-example-reviewer",
    );
    expect(annotations[3].responses?.[0]).toMatchObject({
      outcome: "approved",
      writer: { actorId: "actor:agent:pointbreak-example-author" },
    });
    expect(
      annotations.slice(4, 6).map((annotation) => annotation.status),
    ).toEqual(["replaced", "current"]);

    const routed = renderDecisionContext(annotations);
    expect(routed).toContain(`Decision context (${annotations.length})`);
    for (const annotation of annotations) {
      expect(routed).toContain(`data-anno="${annotation.id}"`);
    }
    expect(routed).toContain("actor:agent:pointbreak-example-author");
    expect(routed).toContain("actor:agent:pointbreak-example-reviewer");
  });

  it("renders uncertainty without fabricating a winner or Git liveness", () => {
    const annotations = compositeAnnotations(uncertainty);
    expect(
      annotations
        .filter((annotation) => annotation.kind === "input-request")
        .map((annotation) => annotation.status),
    ).toEqual(["open", "responded", "ambiguous"]);
    expect(
      annotations
        .filter((annotation) => annotation.kind === "assessment")
        .map((annotation) => annotation.status),
    ).toEqual(["current", "current"]);
    expect(
      annotations
        .filter((annotation) => annotation.kind === "validation")
        .map((annotation) => [annotation.status, annotation.continuity]),
    ).toEqual([
      ["passed", "current"],
      ["failed", "outstanding"],
      ["errored", "outstanding"],
      ["skipped", "skipped"],
      ["failed", "resolved_by_later_pass"],
      ["passed", "current"],
    ]);

    const association = renderAssociationAndLanding(
      uncertainty.commitRange,
      uncertainty.diagnostics,
    );
    expect(association).toContain(
      "landing unknown — Git reachability unavailable",
    );
    expect(association).toContain("withdrawn commits");
    expect(association).toContain("withdrawn refs");
    expect(uncertainty.currentAssessment?.status).toBe("ambiguous");
    expect(uncertainty.currentAssessment?.candidates).toHaveLength(2);
    expect(uncertainty.revisionSupersession?.heads).toHaveLength(2);
    expect(uncertainty.revisionSupersession).not.toHaveProperty("selectedHead");
  });

  it("indexes both the summary heading and derived provenance while retaining immutable ids", () => {
    const member = uncertainty.revision;
    expect(member).toBeDefined();
    const revision: Revision = {
      revisionId: member?.id,
      snapshotId: member?.objectId,
      summary: member?.summary,
      targetDisplay: member?.targetDisplay,
    };
    const index = revisionSearchIndex(revision, {
      state: "head",
      competing: true,
    });
    expect(index.text).toContain("lock decision continuity");
    expect(index.text).toContain(
      "working-tree changes on feat/decision-matrix",
    );
    expect(index.text).toContain(member?.id);
    expect(index.text).toContain(member?.objectId);
    expect(index.is).toContain("contested");
  });
});
