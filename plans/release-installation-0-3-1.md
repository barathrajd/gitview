# License, local installation, and release automation for 0.3.1

This patch release adds explicit MIT licensing and a simple current-user
installation route for Gitview's macOS Apple Silicon release.

## Objective

Let users understand Gitview's reuse terms and install the released binary into
their local executable directory without requiring `sudo`.

## Scope

- Add the MIT license.
- Document the upstream GAH installer and the command to install Gitview from
  its GitHub release into `~/.local/bin`.
- Update package and release metadata to version 0.3.1.
- Set up Release Please to open release pull requests from Conventional
  Commits, and upload the macOS Apple Silicon archive after a release is
  created.

## Acceptance criteria

- [x] The repository contains the standard MIT license text.
- [x] The README documents installing GAH and Gitview for the current user.
- [x] Cargo metadata, CLI output, and release archive report `0.3.1`.
- [x] GAH installs the released binary into `~/.local/bin` on macOS Apple
  Silicon (using GNU `find` from Homebrew `findutils`).
- [x] Release Please configuration and GitHub Actions workflow are published.
- [x] The v0.3.1 GitHub release has the matching title/tag, archive, and
  checksum.
- [x] The v0.3.1 release archive contains only the Gitview executable.
- [x] GAH installation from the single-executable archive was verified by the
  user on macOS Apple Silicon.

## Release boundary

Ship GitHub source archives and a macOS Apple Silicon archive containing only
the executable, plus its checksum. Linux release packaging and Windows support
remain deferred.
