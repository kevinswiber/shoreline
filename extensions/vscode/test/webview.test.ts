import { readFileSync } from "node:fs";
import { build } from "esbuild";
import { expect, it } from "vitest";
import snapshotFixture from "../../../src/cli/inspect/web/test/fixtures/snapshot.json";
import { isHostToWebview, isWebviewToHost } from "../src/webviewProtocol";

const browserEntry = readFileSync("src/webview/review.ts", "utf8");
const protocol = readFileSync("src/webviewProtocol.ts", "utf8");
const theme = readFileSync("src/webview/review.css", "utf8");

it("keeps browser messages on one pure protocol", () => {
  expect(protocol).toContain('type: "render"');
  expect(protocol).toContain('type: "focus"');
  expect(protocol).toContain('type: "error"');
  expect(protocol).toContain('type: "freshness"');
  expect(protocol).toContain('type: "ready"');
  expect(protocol).toContain('type: "openSource"');
  expect(protocol).toContain('type: "reload"');
  expect(browserEntry).toContain('from "../webviewProtocol"');
});

it("validates the complete host and webview message unions", () => {
  expect(
    isHostToWebview({
      type: "focus",
      focus: { kind: "attention", id: "attention:stale" },
    }),
  ).toBe(true);
  expect(isHostToWebview({ type: "focus" })).toBe(true);
  expect(
    isHostToWebview({
      type: "render",
      data: {
        revisionId: "rev:sha256:one",
        snapshotId: "obj:sha256:one",
        artifact: snapshotFixture,
        annotations: [],
      },
      focus: { kind: "attention", id: "attention:stale" },
    }),
  ).toBe(true);
  expect(
    isHostToWebview({
      type: "focus",
      focus: { kind: "attention", id: 42 },
    }),
  ).toBe(false);
  expect(isHostToWebview({ type: "focus", unexpected: true })).toBe(false);
  expect(
    isHostToWebview({
      type: "render",
      data: {},
    }),
  ).toBe(false);
  expect(isWebviewToHost({ type: "ready" })).toBe(true);
  expect(isWebviewToHost({ type: "reload" })).toBe(true);
  expect(
    isWebviewToHost({
      type: "openSource",
      target: {
        filePath: "src/lib.rs",
        side: "new",
        startLine: 1,
        endLine: 2,
      },
    }),
  ).toBe(true);
  expect(isWebviewToHost({ type: "ready", token: "secret" })).toBe(false);
  expect(
    isWebviewToHost({
      type: "openSource",
      target: {
        filePath: "/private/repo/src/lib.rs",
        side: "new",
        startLine: 1,
        endLine: 2,
      },
    }),
  ).toBe(false);
});

it("keeps the complete webview closure presentation-only", async () => {
  const webviewSource = await bundledWebviewSource();

  expect(webviewSource).not.toMatch(
    /\bfetch\s*\(|\bXMLHttpRequest\b|\bWebSocket\b|\bEventSource\b/,
  );
  expect(webviewSource).not.toMatch(/from ["']node:/);
  expect(webviewSource).not.toMatch(
    /\bAuthorization\b|\bBearer\b|\bsessionStorage\b|\bSecretStorage\b|pointbreak\.inspect-startup/,
  );
  expect(browserEntry).toContain("ReviewWebviewController");
});

it("bridges light, dark, and high-contrast themes through VS Code tokens", () => {
  expect(theme).toContain("body.vscode-light");
  expect(theme).toContain("body.vscode-dark");
  expect(theme).toContain("body.vscode-high-contrast");
  expect(theme).toContain("--vscode-editor-background");
  expect(theme).toContain("--vscode-diffEditor-insertedLineBackground");
  expect(theme).toContain("--vscode-diffEditor-removedLineBackground");
  expect(theme).toContain("--vscode-focusBorder");
});

async function bundledWebviewSource(): Promise<string> {
  const result = await build({
    entryPoints: ["src/webview/review.ts"],
    bundle: true,
    outfile: "out/review.js",
    write: false,
    metafile: true,
    platform: "browser",
    format: "iife",
  });

  return Object.keys(result.metafile.inputs)
    .filter((path) => path.startsWith("src/"))
    .map((path) => readFileSync(path, "utf8"))
    .join("\n");
}
