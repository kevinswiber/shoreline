import { describe, expect, it, vi } from "vitest";
import type { WorkspaceFolder } from "vscode";
import revisionFixture from "../../../src/cli/inspect/web/test/fixtures/revision.json";
import snapshotFixture from "../../../src/cli/inspect/web/test/fixtures/snapshot.json";
import {
  compositeAnnotations,
  DiffDataSourceError,
  InspectApiDiffDataSource,
} from "../src/diffDataSource";
import { type FetchFn, InspectClient } from "../src/inspectClient";
import type { ResolvedTargetResolution } from "../src/targetResolver";
import { VERSION_DOC } from "./fixtures";
import { workspaceFolder } from "./helpers/vscodeMock";

describe("InspectApiDiffDataSource", () => {
  it("binds the public revision composite to its snapshot and annotations", async () => {
    const fetch = vi
      .fn<FetchFn>()
      .mockResolvedValueOnce(response(VERSION_DOC))
      .mockResolvedValueOnce(response(revisionFixture))
      .mockResolvedValueOnce(response(snapshotFixture));
    const client = new InspectClient(
      "http://127.0.0.1:63831",
      "secret-bearer",
      fetch,
    );
    const ensure = vi.fn(async () => ({ targetKey: "store/context", client }));
    const source = new InspectApiDiffDataSource({ ensure });
    const target = resolution();

    const data = await source.load({
      resolution: target,
      revisionId: revisionFixture.revision.id,
    });

    expect(ensure).toHaveBeenCalledWith(target);
    expect(
      fetch.mock.calls.map(([url]) => `${url.pathname}${url.search}`),
    ).toEqual([
      "/api/version",
      `/api/revisions/${encodeURIComponent(revisionFixture.revision.id)}`,
      `/api/snapshots/${encodeURIComponent(revisionFixture.revision.objectId)}?contentHash=${encodeURIComponent(revisionFixture.revision.objectArtifactContentHash)}`,
    ]);
    expect(data).toEqual({
      revisionId: revisionFixture.revision.id,
      snapshotId: revisionFixture.revision.objectId,
      artifact: snapshotFixture,
      annotations: compositeAnnotations(revisionFixture),
    });
    expect(JSON.stringify(data)).not.toMatch(
      /secret-bearer|127\.0\.0\.1|63831|\/private\/repo/,
    );
  });

  it("fails before snapshot fetch when the revision has no object identity", async () => {
    const revision = {
      ...revisionFixture,
      revision: { ...revisionFixture.revision, objectId: undefined },
    };
    const fetch = vi
      .fn<FetchFn>()
      .mockResolvedValueOnce(response(VERSION_DOC))
      .mockResolvedValueOnce(response(revision));
    const client = new InspectClient(
      "http://127.0.0.1:63831",
      "secret-bearer",
      fetch,
    );
    const source = new InspectApiDiffDataSource({
      ensure: vi.fn(async () => ({ targetKey: "store/context", client })),
    });

    const error = await source
      .load({
        resolution: resolution(),
        revisionId: revisionFixture.revision.id,
      })
      .catch((caught) => caught);

    expect(error).toBeInstanceOf(DiffDataSourceError);
    expect(error.message).toBe(
      "Pointbreak revision does not identify a captured snapshot.",
    );
    expect(fetch).toHaveBeenCalledTimes(2);
  });
});

describe("compositeAnnotations", () => {
  it("matches the inspector mapping for bodies, tracks, tags, request metadata, and targets", () => {
    const target = {
      kind: "range",
      revisionId: "rev:one",
      filePath: "src/lib.rs",
      side: "new",
      startLine: 7,
      endLine: 8,
    };
    const document = {
      schema: "pointbreak.review-revision",
      version: 2,
      revision: { id: "rev:one", objectId: "obj:one" },
      observations: [
        {
          id: "obs:one",
          title: "Observed",
          body: "**body**",
          bodyContentType: "text/markdown",
          trackId: "agent:author",
          status: "active",
          tags: ["security", "api"],
          target,
        },
      ],
      inputRequests: [
        {
          id: "request:one",
          title: "Choose",
          body: "Need input",
          bodyContentType: "text/plain",
          trackId: "agent:reviewer",
          mode: "operative",
          reasonCode: "manual_decision_required",
          status: "open",
          target: { kind: "revision", revisionId: "rev:one" },
        },
      ],
      assessments: [
        {
          id: "assess:one",
          assessment: "accepted_with_follow_up",
          summary: "Ship with follow-up",
          summaryContentType: "text/markdown",
          trackId: "human:owner",
          status: "current",
          target: { kind: "revision", revisionId: "rev:one" },
        },
      ],
    };

    expect(compositeAnnotations(document)).toEqual([
      {
        kind: "observation",
        id: "obs:one",
        title: "Observed",
        body: "**body**",
        bodyContentType: "text/markdown",
        track: "agent:author",
        tags: ["security", "api"],
        target,
        status: "active",
      },
      {
        kind: "input-request",
        id: "request:one",
        title: "Choose",
        body: "Need input",
        bodyContentType: "text/plain",
        track: "agent:reviewer",
        tags: [],
        target: { kind: "revision", revisionId: "rev:one" },
        status: "open",
        mode: "operative",
        reasonCode: "manual_decision_required",
      },
      {
        kind: "assessment",
        id: "assess:one",
        title: "assessment: accepted-with-follow-up",
        body: "Ship with follow-up",
        bodyContentType: "text/markdown",
        track: "human:owner",
        tags: [],
        target: { kind: "revision", revisionId: "rev:one" },
        status: "current",
        assessment: "accepted_with_follow_up",
      },
    ]);
  });
});

function resolution(): ResolvedTargetResolution {
  return {
    kind: "resolved",
    folder: workspaceFolder("/private/repo", "repo") as WorkspaceFolder,
    target: {
      key: "store/context",
      label: "repo",
      storeIdentity: "store",
      contextIdentity: "context",
    },
    emptyInventory: false,
  };
}

function response(document: unknown) {
  return {
    status: 200,
    text: async () => JSON.stringify(document),
  };
}
