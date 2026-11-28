---
id: TASK-42
title: 'Nix module: add a port option to the cairn serve launchd agent'
status: Done
assignee:
  - '@claude'
created_date: '2026-11-28T00:16:39Z'
updated_date: '2026-11-28T00:18:20Z'
labels: []
dependencies: []
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
nix/home-module.nix hardcodes `serve --port 0`, so the launchd agent always listens on random ports. Users need to choose the web UI port, with the normal `cairn serve` port as the default. This changes current behaviour: an existing install moves from random ports to 7878 unless it sets the port to 0. The content port needs no option of its own here: `cairn serve` already derives it (UI port + 1, or a free port when the UI port is 0). The same change is being made for scree (SCR-38).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 services.cairn.port is passed to cairn serve as --port and defaults to 7878, the cairn serve default
- [x] #2 Setting the port to another value, including 0 for random free ports, produces the matching --port argument (checked by evaluating the module)
- [x] #3 README.md, docs/architecture.md, the module header comment and the extraArgs description describe the port option and its default
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Add services.cairn.port (lib.types.port, default 7878) to nix/home-module.nix and pass it as --port. 2. Update the header comment, extraArgs text, README.md and docs/architecture.md. 3. Evaluate the module with default, custom and 0 ports.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified with nix eval of the module (aarch64-darwin pkgs, stub Home Manager options): default gives --port 7878, 9000 and 0 pass through, 70000 is rejected. nixpkgs-fmt check clean. Not loaded on a real Mac.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added services.cairn.port (default 7878, 0 for free ports), passed to cairn serve as --port. Existing installs move from random ports to 7878 unless they set port = 0. README, architecture doc and module text updated.
<!-- SECTION:FINAL_SUMMARY:END -->
