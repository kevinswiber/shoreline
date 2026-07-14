import type {
  ReviewAssessmentDoc,
  ReviewFactTarget,
  ReviewInputRequestDoc,
  ReviewObservationDoc,
  ReviewSnapshotDoc,
  RevisionDoc,
} from "./cli";
import type { InspectSession } from "./inspectChild";
import type { ResolvedTargetResolution } from "./targetResolver";

export interface Annotation {
  readonly id: string;
  readonly kind: string;
  readonly title: string;
  readonly track: string;
  readonly body?: string;
  readonly bodyContentType?: string;
  readonly tags?: string[];
  readonly target?: ReviewFactTarget;
}

export interface DiffLoadRequest {
  readonly resolution: ResolvedTargetResolution;
  readonly revisionId: string;
}

export interface DiffRenderData {
  readonly revisionId: string;
  readonly snapshotId: string;
  readonly artifact: ReviewSnapshotDoc;
  readonly annotations: Annotation[];
}

export interface DiffDataSource {
  load(request: DiffLoadRequest): Promise<DiffRenderData>;
}

export interface InspectSessionProvider {
  ensure(resolution: ResolvedTargetResolution): Promise<InspectSession>;
}

export class DiffDataSourceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DiffDataSourceError";
  }
}

/** Loads transport-neutral annotated diff data through the active host session. */
export class InspectApiDiffDataSource implements DiffDataSource {
  constructor(private readonly manager: InspectSessionProvider) {}

  async load(request: DiffLoadRequest): Promise<DiffRenderData> {
    const { client } = await this.manager.ensure(request.resolution);
    await client.verifyVersion();
    const revision = await client.revision(request.revisionId);
    const snapshotId = revision.revision.objectId;
    if (!snapshotId) {
      throw new DiffDataSourceError(
        "Pointbreak revision does not identify a captured snapshot.",
      );
    }
    const artifact = await client.snapshot(
      snapshotId,
      revision.revision.objectArtifactContentHash,
    );
    return {
      revisionId: request.revisionId,
      snapshotId,
      artifact,
      annotations: compositeAnnotations(revision),
    };
  }
}

type CompositeFacts = Pick<
  RevisionDoc,
  "observations" | "inputRequests" | "assessments"
>;

/** Maps public revision facts exactly as the bundled inspector does. */
export function compositeAnnotations(document: CompositeFacts): Annotation[] {
  return [
    ...document.observations.map(observationAnnotation),
    ...document.inputRequests.map(inputRequestAnnotation),
    ...document.assessments.map(assessmentAnnotation),
  ];
}

function observationAnnotation(observation: ReviewObservationDoc): Annotation {
  return {
    kind: "observation",
    id: observation.id ?? "",
    title: observation.title ?? "(observation)",
    body: observation.body ?? "",
    bodyContentType: observation.bodyContentType,
    track: observation.trackId ?? "",
    tags: Array.isArray(observation.tags) ? observation.tags : [],
    target: observation.target ?? {},
  };
}

function inputRequestAnnotation(request: ReviewInputRequestDoc): Annotation {
  const metadata = [request.mode, request.reasonCode]
    .filter(Boolean)
    .join(" · ");
  return {
    kind: "input-request",
    id: request.id ?? "",
    title: request.title ?? "(input request)",
    body: request.body ?? "",
    bodyContentType: request.bodyContentType,
    track: request.trackId ?? "",
    tags: metadata ? [metadata] : [],
    target: request.target ?? {},
  };
}

function assessmentAnnotation(assessment: ReviewAssessmentDoc): Annotation {
  const label = assessmentDisplayLabel(assessment.assessment ?? "");
  return {
    kind: "assessment",
    id: assessment.id ?? "",
    title: `assessment: ${label || "?"}`,
    body: assessment.summary ?? "",
    bodyContentType: assessment.summaryContentType,
    track: assessment.trackId ?? "",
    tags: [],
    target: assessment.target ?? {},
  };
}

function assessmentDisplayLabel(value: string): string {
  return ASSESSMENT_LABELS[value] ?? value;
}

const ASSESSMENT_LABELS: Readonly<Record<string, string>> = {
  accepted: "accepted",
  accepted_with_follow_up: "accepted-with-follow-up",
  needs_changes: "needs-changes",
  needs_clarification: "needs-clarification",
};
