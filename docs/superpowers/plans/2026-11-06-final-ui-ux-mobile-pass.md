# Final UI/UX and mobile pass — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship roadmap #8 as three PRs: mascot moments, mobile pass, light polish.

**Architecture:** Extend `EmptyState` with optional WebP illustrations; commit resized assets under `apps/web/public/mascot/`; wire moment call sites; then responsive fixes; then polish and roadmap move.

**Tech Stack:** React, Mantine, TanStack Router, CSS modules, `cwebp`.

---

### Task 1: Spec on branch

- [ ] Branch `feat/ui-mascot-moments` from main
- [ ] Spec + this plan committed

### Task 2: Assets + EmptyState + mascot module

- [ ] `mkdir -p apps/web/public/mascot` and generate WebPs from `brand/graphics/`
- [ ] `components/mascot/mascotAssets.ts` + optional `MascotFigure`
- [ ] Extend `EmptyState` with `illustration`

### Task 3: Wire moments + 404 + login/onboarding

- [ ] Library grid + explorer, trash, shared page-level empty
- [ ] NotFound page + root `notFoundComponent`
- [ ] Login wide panel; onboarding standing Dean
- [ ] Tests, prettier, tsc; FEATURES bullet; push

### Task 4: Mobile pass (new branch)

- [ ] Drawers, tables, library chrome, auth pages
- [ ] Push `feat/ui-mobile-pass`

### Task 5: Polish + close #8

- [ ] Spacing/copy nits; ROADMAP → FEATURES; Remaining for welcome email
- [ ] `graphify update .`; push `feat/ui-ux-polish`
