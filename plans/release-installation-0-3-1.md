# License and local installation for 0.3.1

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

## Acceptance criteria

- [x] The repository contains the standard MIT license text.
- [x] The README documents installing GAH and Gitview for the current user.
- [x] Cargo metadata, CLI output, and release archive report `0.3.1`.
- [ ] GAH installs the released binary into `~/.local/bin` on macOS Apple
  Silicon.

## Release boundary

Ship GitHub source archives and the macOS Apple Silicon archive. Linux release
packaging and Windows support remain deferred.
