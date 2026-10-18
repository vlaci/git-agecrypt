<!-- SPDX-FileCopyrightText: 2026 László Vaskó <opensource@vlaci.email>

SPDX-License-Identifier: MPL-2.0 -->

# Changelog

## [0.6.0] - 2026-10-18

### Security
- Changing recipients in `git-agecrypt.toml` now takes effect for files whose
  content hasn't changed. After editing recipients, run
  `git add --renormalize .` and commit.
- Files with the same base name and differing extenison no longer
  share a cache entry. Previously one of them could be committed encrypted to
  the other file's recipients.
- Updated `age` to 0.10.1 (RUSTSEC-2024-0433; 0.10.0 is yanked) and `git2` to
  0.20.4, and refreshed the remaining dependencies.

### Added
- `git-agecrypt rekey [PATHS...]`: re-encrypts files to the recipients in
  `git-agecrypt.toml` and stages them. Without arguments it covers every
  tracked file that uses the `git-agecrypt` filter.

### Fixed
- Identities are read from only the `git-agecrypt.config.identity`
  key. Previously any git config key containing "identity" (for
  example `advice.implicitIdentity`) was used as an identity path,
  which broke checkout.
- The first `git add` in a repository with no commits no longer fails.
- Clean no longer panics when a path is a directory in `HEAD`.
- The Nix package builds again on current nixpkgs, including darwin.

### Changed
- The cache moved to `.git/git-agecrypt/v2/`: one file per path, written
  atomically and readable only by the owner. An unreadable entry is ignored
  instead of failing `git add`.

### Upgrade notes
- Old `.git/git-agecrypt/*.hash` and `*.age` files are ignored and can be
  deleted.
- If you changed recipients with an earlier version, run
  `git-agecrypt rekey` once and commit, so existing files are re-encrypted to
  the current recipients.
- Removed recipients can still decrypt older blobs in history. To revoke
  access, rotate the secret itself.
- Nix flake: `overlay` is deprecated in favour of `overlays.default`. The
  flake no longer has a `flake-utils` input, so drop any
  `inputs.git-agecrypt.inputs.flake-utils.follows`. Packages are built for
  x86_64-linux, aarch64-linux and aarch64-darwin only.

## [0.2.0]
- Initial version.
