import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { releases } from "../data/releases";

// Release Please updates this manifest when a release is cut. Cargo.toml can
// already contain the next development version, which isn't installable yet.
const manifest = JSON.parse(readFileSync(resolve(process.cwd(), "../.release-please-manifest.json"), "utf8"));
if (typeof manifest["."] !== "string") throw new Error("Could not read the released version from .release-please-manifest.json");

/** The version of Gitview the install options give, such as "0.7.0". */
export const currentVersion = manifest["."] as string;

/** The release notes for the current version. */
export const currentRelease = releases.find((release) => release.version === currentVersion);
if (!currentRelease) {
  throw new Error(
    `website/src/data/releases.ts has no entry for ${currentVersion}; add one before releasing.`,
  );
}
