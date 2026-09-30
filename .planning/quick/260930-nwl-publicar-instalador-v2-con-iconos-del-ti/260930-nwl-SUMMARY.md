---
status: complete
date: 2026-09-30
source_commit: 3e217e3fe7c37c778fddc53fcb817331461228e4
---

# Branded V2 Windows installer release

- Replaced the actual generic template ICO (visually verified before modification) with approved full shark artwork at 16/24/32/48/64/128/256 px, preserving alpha. Reproducible conversion: scripts/build-icon.ps1, called by packaging.
- App and setup extracted icons visually verified as the full shark. Native product/file version and Cargo version now 2.0.0.
- Inno includes unchanged project GNU GPL v3 in license acceptance page, installs LICENSE beside app, English/Spanish wizard, optional desktop shortcut, uninstall display icon and per-user installation. No new proprietary EULA invented.
- 22 automated tests pass; 1 physical test intentionally ignored in standard suite. Release, portable and installer builds passed. Installer 3,190,524 bytes.
- Personal H application safely restarted with rebuilt EXE (previous version kept in pre-shark-release backup); installer copied to H folder. History/startup settings preserved. No full interactive install/uninstall smoke claimed.
- Source pushed without force to main at 3e217e3; v2.0.0 release published after uploads verified. Tag provides matching complete source for GPL distribution. Installer and SHA256SUMS.txt attached; GitHub asset digests match local SHA256.
- Release: https://github.com/lferraro1103/r5-battery-estimator/releases/tag/v2.0.0
- Installer SHA256: df2a2c739dcd415969b62aacc786011e49662dcf2ca7d310b1647103ae1ce438
- Unsigned binaries honestly disclosed in bilingual release notes; automatic configuration profiles remain explicitly pending. No claims about SmartScreen certification or unavailable functionality.

Workflow ran inline sequential under the documented Codex single-dispatch isolation fallback; no unnecessary parallel agent overhead.
