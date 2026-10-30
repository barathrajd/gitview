import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { releases } from "../data/releases";

// The site is built from the `website` folder, locally and on CI, so the
// crate's manifest is one level up. Reading it at build time keeps every
// version on the site in step with the release that was just cut.
const manifest = readFileSync(resolve(process.cwd(), "../Cargo.toml"), "utf8");
const found = manifest.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m);
if (!found) throw new Error("Could not read the package version from Cargo.toml");

/** The version of Gitview the install options give, such as "0.7.0". */
export const currentVersion = found[1];

/** The release notes for the current version. */
export const currentRelease = releases.find((release) => release.version === currentVersion);
if (!currentRelease) {
  throw new Error(
    `website/src/data/releases.ts has no entry for ${currentVersion}; add one before releasing.`,
  );
}
