import { beforeEach, describe, expect, it } from "vitest";
import {
  connectionPresentation,
  getConnectionSnapshot,
  markRequestFailure,
  markRequestSuccess,
  resetConnectionState,
  setRefreshState,
} from "../src/connection";

beforeEach(() => {
  resetConnectionState();
});

describe("independent connection and refresh state", () => {
  it("starts with neutral connecting chrome", () => {
    expect(getConnectionSnapshot()).toEqual({
      connection: "connecting",
      refresh: "idle",
    });
    expect(connectionPresentation(getConnectionSnapshot())).toMatchObject({
      serverLabel: "local server",
      connectionLabel: "connecting",
      action: null,
    });
  });

  it("classifies unauthorized, unreachable, and protocol failures exactly", () => {
    markRequestFailure("unauthorized");
    expect(connectionPresentation(getConnectionSnapshot())).toMatchObject({
      connectionLabel: "authentication required",
      action: "Reconnect",
      canConnectAnother: false,
    });

    resetConnectionState();
    markRequestFailure("unreachable");
    expect(connectionPresentation(getConnectionSnapshot())).toMatchObject({
      connectionLabel: "server unavailable",
      action: "Retry",
      canConnectAnother: true,
    });

    resetConnectionState();
    markRequestFailure("protocol");
    expect(getConnectionSnapshot()).toEqual({
      connection: "connected",
      refresh: "degraded",
    });
    expect(connectionPresentation(getConnectionSnapshot())).toMatchObject({
      connectionLabel: "connected",
      refreshLabel: "response error",
      action: "Retry",
      canConnectAnother: false,
    });
  });

  it("does not collapse refresh activity into connection state", () => {
    markRequestSuccess();
    setRefreshState("updated");
    expect(getConnectionSnapshot()).toEqual({
      connection: "connected",
      refresh: "updated",
    });
    markRequestFailure("unauthorized");
    expect(getConnectionSnapshot()).toEqual({
      connection: "unauthorized",
      refresh: "updated",
    });
  });
});
