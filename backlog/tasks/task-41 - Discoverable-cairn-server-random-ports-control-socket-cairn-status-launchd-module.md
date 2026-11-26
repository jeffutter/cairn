---
id: TASK-41
title: >-
  Discoverable cairn server: random ports, control socket, cairn status, launchd
  module
status: Done
assignee:
  - '@claude'
created_date: '2026-11-26T13:00:43Z'
updated_date: '2026-11-26T13:04:49Z'
labels: []
dependencies: []
type: feature
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Run cairn serve as a background service on random ports and let the CLI discover its URL over a unix socket, so Agents can return a full URL for what they submitted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 serve --port 0 binds two OS-assigned ports and the UI and content origins work on them
- [x] #2 serve binds a unix socket beside the database and removes it on SIGINT and SIGTERM
- [x] #3 cairn status asks the socket for the running server's URL and ports, and exits 3 when none runs
- [x] #4 cairn view prints the running server's URL when --url and CAIRN_URL are unset
- [x] #5 The skill tells Agents to use cairn status and cairn view to return full URLs
- [x] #6 The flake exports a home-manager module with a reusable launchd agent
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. src/control.rs: socket path = database path with extension sock; lock file for ownership; one-line JSON request/response
2. serve: allow port 0, bind then read real ports, handle SIGTERM, --advertise-host
3. cli: status command; view consults the socket (--url/CAIRN_URL still win)
4. skill prose, docs, ADR 0009, snapshot
5. flake homeManagerModules.default with launchd.agents.cairn
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified: 287 nextest tests, nix flake check on x86_64-linux, manual serve --port 0 / status / view / curl 200. launchd module evaluated for aarch64-darwin (generated agent config inspected) and rejected on linux by its assertion; not loaded on a real Mac.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
serve accepts --port 0, listens on <db>.sock (ADR 0009), and handles SIGTERM. New cairn status; cairn view uses the running server URL. New --advertise-host. Skill tells Agents to return the view URL. Flake exports homeManagerModules.default (services.cairn, macOS launchd).
Risk: the launchd agent has not been run on a Mac.
<!-- SECTION:FINAL_SUMMARY:END -->
