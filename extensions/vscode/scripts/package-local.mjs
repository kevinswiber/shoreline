import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  assertExactArchiveFiles,
  assertExactPackageFiles,
  powershellCommand,
  verifyBundledBinary,
} from "./package-contract.mjs";

const scriptRoot = path.dirname(fileURLToPath(import.meta.url));
const extensionRoot = path.resolve(scriptRoot, "..");
const repoRoot = path.resolve(extensionRoot, "../..");
const targetManifest = JSON.parse(
  readFileSync(path.join(repoRoot, ".github/binary-targets.json"), "utf8"),
);
const hostLabel = `${process.platform}-${process.arch}`;
const knownLabels = new Set(targetManifest.map(({ target }) => target));
const hostTarget = targetManifest.find(({ target }) => target === hostLabel);
if (!hostTarget) {
  throw new Error(
    `Unsupported packaging host ${hostLabel}; expected one of ${[...knownLabels].join(", ")}`,
  );
}

const executable = hostTarget.executable;
const configuredBinary = process.env.POINTBREAK_EXTENSION_BINARY?.trim();
const sourceBinary = configuredBinary
  ? path.resolve(configuredBinary)
  : path.join(repoRoot, "target", "release", executable);
const bundledRelative = `bin/${hostLabel}/${executable}`;
const bundledBinary = path.join(extensionRoot, bundledRelative);

if (!configuredBinary) {
  run("cargo", ["build", "--release", "--bin", "pointbreak"], repoRoot);
} else if (!existsSync(sourceBinary)) {
  throw new Error(
    `Configured Pointbreak executable does not exist: ${sourceBinary}`,
  );
}

const approvedEvidence = binaryEvidence(sourceBinary);
const expectedSha256 = process.env.POINTBREAK_EXTENSION_BINARY_SHA256?.trim();
if (
  expectedSha256 &&
  approvedEvidence.sha256 !== expectedSha256.toLowerCase()
) {
  throw new Error(
    `Approved Pointbreak executable SHA-256 mismatch: expected ${expectedSha256.toLowerCase()}, received ${approvedEvidence.sha256}`,
  );
}
mkdirSync(path.dirname(bundledBinary), { recursive: true });
copyFileSync(sourceBinary, bundledBinary);
const bundledEvidence = binaryEvidence(bundledBinary);
verifyBundledBinary(approvedEvidence, bundledEvidence);

for (const entry of readdirSync(extensionRoot)) {
  if (entry.startsWith("pointbreak-") && entry.endsWith(".vsix")) {
    rmSync(path.join(extensionRoot, entry));
  }
}

run("npm", ["run", "build"], extensionRoot);
assertListedFiles(bundledRelative);
run("npx", ["--no-install", "vsce", "package"], extensionRoot);

const artifacts = readdirSync(extensionRoot).filter(
  (entry) => entry.startsWith("pointbreak-") && entry.endsWith(".vsix"),
);
if (artifacts.length !== 1) {
  throw new Error(
    `Expected one packaged VSIX, found ${artifacts.length}: ${artifacts.join(", ")}`,
  );
}
const artifact = path.join(extensionRoot, artifacts[0]);
assertArchiveFiles(artifact, bundledRelative);
const archivedEvidence = {
  sha256: archiveBinarySha256(artifact, bundledRelative),
  versionDocument: bundledEvidence.versionDocument,
};
verifyBundledBinary(approvedEvidence, archivedEvidence);
console.log(
  JSON.stringify(
    {
      vsix: artifact,
      bundledBinary: {
        path: bundledRelative,
        sha256: archivedEvidence.sha256,
        version: JSON.parse(archivedEvidence.versionDocument),
      },
    },
    null,
    2,
  ),
);

function assertListedFiles(binary) {
  const result = run(
    "npx",
    ["--no-install", "vsce", "ls"],
    extensionRoot,
    true,
  );
  assertExactPackageFiles(
    result.stdout.split(/\r?\n/).filter(Boolean),
    binary,
    "vsce ls",
  );
}

function assertArchiveFiles(artifact, binary) {
  const files = archiveEntries(artifact)
    .split(/\r?\n/)
    .filter((entry) => entry && !entry.endsWith("/"));
  assertExactArchiveFiles(files, binary, "VSIX archive");
}

function binaryEvidence(binary) {
  const versionDocument = run(
    binary,
    ["version", "--format", "json"],
    repoRoot,
    true,
  ).stdout.trim();
  let version;
  try {
    version = JSON.parse(versionDocument);
  } catch {
    throw new Error(
      `Pointbreak executable returned invalid version JSON: ${binary}`,
    );
  }
  if (
    version.schema !== "pointbreak.version" ||
    version.version !== 1 ||
    typeof version.cliVersion !== "string" ||
    typeof version.documents !== "object" ||
    version.documents === null
  ) {
    throw new Error(
      `Pointbreak executable returned an incompatible machine identity: ${binary}`,
    );
  }
  return { sha256: sha256File(binary), versionDocument };
}

function sha256File(file) {
  return createHash("sha256").update(readFileSync(file)).digest("hex");
}

function archiveBinarySha256(artifact, binary) {
  const entry = `extension/${binary}`;
  if (process.platform !== "win32") {
    const result = run(
      "unzip",
      ["-p", artifact, entry],
      extensionRoot,
      true,
      null,
    );
    return createHash("sha256").update(result.stdout).digest("hex");
  }
  const hashScript = powershellCommand(
    [
      "Add-Type -AssemblyName System.IO.Compression.FileSystem",
      "$archive = [System.IO.Compression.ZipFile]::OpenRead($args[0])",
      "$entry = $archive.GetEntry($args[1])",
      'if ($null -eq $entry) { throw "missing archive entry $($args[1])" }',
      "$stream = $entry.Open()",
      "$sha = [System.Security.Cryptography.SHA256]::Create()",
      'try { [BitConverter]::ToString($sha.ComputeHash($stream)).Replace("-", "").ToLowerInvariant() } finally { $sha.Dispose(); $stream.Dispose(); $archive.Dispose() }',
    ].join("; "),
  );
  return run(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", hashScript, artifact, entry],
    extensionRoot,
    true,
  ).stdout.trim();
}

function archiveEntries(artifact) {
  if (process.platform !== "win32") {
    return run("unzip", ["-Z1", artifact], extensionRoot, true).stdout;
  }
  const listScript = powershellCommand(
    [
      "Add-Type -AssemblyName System.IO.Compression.FileSystem",
      "$archive = [System.IO.Compression.ZipFile]::OpenRead($args[0])",
      "try { $archive.Entries | ForEach-Object FullName } finally { $archive.Dispose() }",
    ].join("; "),
  );
  return run(
    "powershell.exe",
    ["-NoProfile", "-NonInteractive", "-Command", listScript, artifact],
    extensionRoot,
    true,
  ).stdout;
}

function run(command, args, cwd, capture = false, encoding = "utf8") {
  const result = spawnSync(command, args, {
    cwd,
    encoding,
    // Captured stdout has to hold a whole binary when archiveBinarySha256 pipes
    // `unzip -p` of the bundled executable. An unstripped debug build exceeds the
    // former 64 MiB ceiling, so double it. Revisit if release binaries approach this.
    maxBuffer: 128 * 1024 * 1024,
    stdio: capture ? ["ignore", "pipe", "pipe"] : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    const stderr = Buffer.isBuffer(result.stderr)
      ? result.stderr.toString("utf8").trim()
      : result.stderr?.trim();
    throw new Error(
      `${command} ${args.join(" ")} failed with exit code ${result.status}${stderr ? `: ${stderr}` : ""}`,
    );
  }
  return result;
}
