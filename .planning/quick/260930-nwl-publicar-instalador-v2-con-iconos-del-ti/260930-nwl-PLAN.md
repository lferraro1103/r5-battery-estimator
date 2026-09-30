---
mode: quick
autonomous: true
---

# Publish native V2 installer with shark branding and GPL v3

Execution: inline sequential, per Codex GSD single-dispatch isolation fallback. Live inspection confirmed icon.ico is the generic template, while assets/shark-battery.png is the approved full shark. Preserve unrelated untracked files and all battery/UI behavior.

1. Convert the existing full shark PNG into a multi-resolution Windows ICO; keep a reproducible build script. Embed it in app/setup/shortcuts. Align release version 2.0.0 and add the existing GPL v3 LICENSE to installer acceptance page, Spanish/English languages.
2. Build/test/package; verify actual embedded EXE and setup icons. Update personal H executable safely and create SHA256 checksums. Do not change local history/startup preferences or sign with invented credentials.
3. Push exact source to main without force, publish a new GitHub v2.0.0 release with installer and checksums, verified by API. Include source/tag and known pending profiles/unsigned Windows warning in release notes. Never overwrite an existing release asset silently.

Threat model: authentication token stays in process memory, not logs or repo; release upload uses HTTPS to GitHub for this repository only. Installer per-user, no elevated privileges, no telemetry. GPL text unchanged; binaries accompanied by matching public tagged source. Confirm checksum after upload.
