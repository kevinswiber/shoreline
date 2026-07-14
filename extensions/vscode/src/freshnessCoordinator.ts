import type { Disposable, Event } from "vscode";
import type { InspectFreshnessDoc } from "./cli";
import type { InspectSession } from "./inspectChild";
import { InspectClientError } from "./inspectClient";

const DEFAULT_REFRESH_INTERVAL_MS = 15_000;

export interface FreshnessConnection {
  readonly onDidChangeSession: Event<{ targetKey: string } | undefined>;
  activeSession(): InspectSession | undefined;
}

export interface FreshnessAttention {
  readonly onDidChangeVisibility: Event<boolean>;
  isVisible(): boolean;
  refresh(): Promise<void>;
  refreshTarget(targetKey: string, signal?: AbortSignal): Promise<void>;
}

export interface FreshnessPanels {
  readonly onDidChangeVisibility: Event<boolean>;
  isVisible(): boolean;
  reloadActive(targetKey?: string, signal?: AbortSignal): Promise<void>;
}

interface FreshnessCoordinatorOptions {
  readonly intervalMs?: number;
  readonly reportError?: (error: InspectClientError) => void;
}

interface MarkerBaseline {
  readonly targetKey: string;
  readonly document: InspectFreshnessDoc;
}

/** Gates cold review refreshes behind the active inspect connection's marker. */
export class FreshnessCoordinator implements Disposable {
  private readonly intervalMs: number;
  private readonly reportError: (error: InspectClientError) => void;
  private readonly subscriptions: Disposable[];
  private readonly polling = new Set<number>();
  private timer: ReturnType<typeof setInterval> | undefined;
  private baseline: MarkerBaseline | undefined;
  private reportedError: InspectClientError["kind"] | undefined;
  private abortController = new AbortController();
  private generation = 0;
  private disposed = false;

  constructor(
    private readonly connection: FreshnessConnection,
    private readonly attention: FreshnessAttention,
    private readonly panels: FreshnessPanels,
    options: FreshnessCoordinatorOptions = {},
  ) {
    this.intervalMs = options.intervalMs ?? DEFAULT_REFRESH_INTERVAL_MS;
    this.reportError = options.reportError ?? (() => undefined);
    this.subscriptions = [
      connection.onDidChangeSession((event) => {
        this.reconcile(event?.targetKey);
      }),
      attention.onDidChangeVisibility(() => this.reconcile()),
      panels.onDidChangeVisibility(() => this.reconcile()),
    ];
    this.reconcile();
  }

  async refreshAll(): Promise<void> {
    await this.attention.refresh();
    this.baseline = undefined;
  }

  async refreshAfterWrite(): Promise<void> {
    await Promise.all([this.attention.refresh(), this.panels.reloadActive()]);
    this.baseline = undefined;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    this.invalidate();
    for (const subscription of this.subscriptions) {
      subscription.dispose();
    }
  }

  private reconcile(activatedTarget?: string): void {
    this.invalidate();
    if (this.disposed) return;
    const signal = this.abortController.signal;
    if (activatedTarget) {
      void this.attention.refreshTarget(activatedTarget, signal);
    }
    if (!this.hasVisibleSurface()) return;
    const session = this.connection.activeSession();
    if (!session) return;

    const generation = this.generation;
    void this.poll(generation, session, signal);
    this.timer = setInterval(() => {
      void this.poll(generation, session, signal);
    }, this.intervalMs);
  }

  private async poll(
    generation: number,
    session: InspectSession,
    signal: AbortSignal,
  ): Promise<void> {
    if (
      this.polling.has(generation) ||
      !this.isCurrent(generation, session, signal)
    ) {
      return;
    }
    this.polling.add(generation);
    try {
      const document = await session.client.freshness();
      if (!this.isCurrent(generation, session, signal)) return;
      this.reportedError = undefined;

      const previous = this.baseline;
      this.baseline = { targetKey: session.targetKey, document };
      if (
        !previous ||
        previous.targetKey !== session.targetKey ||
        !markerMoved(previous.document, document)
      ) {
        return;
      }

      await this.attention.refreshTarget(session.targetKey, signal);
      if (!this.isCurrent(generation, session, signal)) return;
      await this.panels.reloadActive(session.targetKey, signal);
    } catch (error) {
      if (!this.isCurrent(generation, session, signal)) return;
      const reported =
        error instanceof InspectClientError
          ? error
          : new InspectClientError("protocol");
      if (reported.kind !== this.reportedError) {
        this.reportedError = reported.kind;
        this.reportError(reported);
      }
    } finally {
      this.polling.delete(generation);
    }
  }

  private isCurrent(
    generation: number,
    session: InspectSession,
    signal: AbortSignal,
  ): boolean {
    return (
      !this.disposed &&
      !signal.aborted &&
      generation === this.generation &&
      this.hasVisibleSurface() &&
      this.connection.activeSession() === session
    );
  }

  private hasVisibleSurface(): boolean {
    return this.attention.isVisible() || this.panels.isVisible();
  }

  private invalidate(): void {
    this.abortController.abort();
    this.abortController = new AbortController();
    this.generation += 1;
    this.baseline = undefined;
    this.reportedError = undefined;
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = undefined;
    }
  }
}

function markerMoved(
  previous: InspectFreshnessDoc,
  current: InspectFreshnessDoc,
): boolean {
  if (previous.eventCount !== current.eventCount) return true;
  if (
    previous.commitGraphStamp === undefined ||
    current.commitGraphStamp === undefined
  ) {
    return false;
  }
  return previous.commitGraphStamp !== current.commitGraphStamp;
}
