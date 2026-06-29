// Build the inspector bundle: esbuild → the committed src/cli/inspect/assets/app.js.
// Run via `npm run build` / `just web-build` after editing web/src. Uses the shared
// esbuild.config.mjs options so the output matches the determinism test exactly.
//
// `npm run build -- --outfile=<path>` overrides the destination (used by the release
// A3 verify, which builds to a scratch dir and compares — it must never mutate the
// committed file). Default destination is the committed bundle.
import { build } from "esbuild";
import { buildOptions } from "./esbuild.config.mjs";

const override = process.argv
  .find((arg) => arg.startsWith("--outfile="))
  ?.slice("--outfile=".length);

await build({ ...buildOptions, outfile: override || "../assets/app.js" });
