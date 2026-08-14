# vstack2 v0.2 — session handoff

Start a fresh session in `/home/method/dev/vstack2` and run this to completion.
Everything through Phase 5 plus the Phase 6 restructure is built, reviewed, and
green; three tasks remain, the first is the headline.

## Where we are

- **Phases 0–6 implemented, green, reviewed.** ~52 commits this cycle, 530 Rust
  tests + the UI suite passing, `tools/guard` clean. Eight adversarial-review
  rounds folded in across the engine/adapter/catalog/safety work.
- **The IA is approved.** The owner reviewed the navigation proposal
  (artifact `7487fa03-dc91-4af2-993d-be808e31c62b`) and said proceed with the
  recommendation. The app is already reorganized 8 rooms → 6: Home (triage),
  Review & apply (was Sync, safety inline), Library (Library+Catalogs merged,
  bundles lead the "add" path), Tools & Projects (merged), Customize, Settings.
- **Remaining:** (1) UI visual polish to Vercel grade — the owner's explicit
  ask; (2) real-app Tauri walkthrough; (3) cut the v0.2.0 release (owner-gated).

## Orient first (read these)

- `docs/PLAN.md` — the progress block at the top is the resume anchor; Phases
  0–5 sections are consumed, 6–7 remain.
- `docs/ARCHITECTURE.md` — the 13 invariants and the durable decisions.
- `docs/adapters/` — per-harness reference (roots, surfaces, formats).
- `tools/guard` — **the gate. Read it; it is the rules list.** It runs
  `cargo fmt`/`clippy -D warnings`/`cargo test`, then `tsc`/`biome`/`vitest`,
  plus size caps and content bans. Every change keeps it green.

### Standing rules and quirks

- **Do not modify anything in `dev/vstack/`** (the v1 repo) — reference only.
- **Commits:** conventional format, subject ≤ 72 chars, body ends with the
  `Claude-Session:` line. A `commit-msg` hook rejects any `crates/`/`ui/`
  change without a `CHANGELOG.md` entry — add one, or put `[no-changelog]` in
  the subject for pure refactors/tests. Branch off `main` only if asked to open
  a PR; otherwise commit to `main`.
- **Subagent reports** may arrive only as idle notifications. Recover the real
  report from `~/.hclaude/projects/<project-slug>/<session-id>/subagents/agent-*.jsonl`
  (take the last assistant text block). Never nudge a wedged agent.
- **New engine/render changes get an adversarial review before merge** (spawn a
  second agent prompted to break the invariants with failing tests). UI-only
  changes skip it.
- Cheap models (Sonnet-class or below) do UI implementation; the main loop
  reviews, it does not pixel-push.

---

## Task 1 (headline): UI polish to Vercel grade

The owner: "Compare to Vercel's dashboard. We're nowhere near that polish and
consistency on the shadcn components. Use some color — not pure black and
white — ultra-clean Vercel-style."

### The problem, precisely

`ui/src/index.css` holds the design tokens, and they are the **stock shadcn
defaults: pure monochrome.** Every color is `oklch(L 0 0)` — chroma `0`, i.e.
grayscale — except `--destructive` (red). There is no brand accent and no
semantic palette. That flat grayscale is the "pure black and white" complaint.
The pages and the 16 shadcn components in `ui/src/components/ui/` are otherwise
stock and inconsistent in spacing, borders, elevation, focus, and radius.

### The target — the owner's Vercel reference

**The owner supplied four Vercel dashboard screenshots as the explicit bar.**
They live at `/home/method/dev/.vstack-design-refs/` (outside the repo, so not
committed): `vercel-deployments.png`, `vercel-env-vars.png`, `vercel-domains.png`,
`vercel-observability.png`. **Open them and study them before touching a token.**
(Ignore the bright-blue rectangle around one sidebar item in two shots — that is
a screenshot focus-ring artifact, not a design element to copy.)

What those screens actually do, extracted for you to match:

- **Ground is true near-black, not grey.** Dark theme sits on ~`oklch(0.10–0.12)`,
  effectively `#0a0a0a`, not the current `oklch(0.145)` washed grey-black. The
  sidebar and content share the ground, separated only by a **hairline border at
  ~8–12% white** — barely visible, never a hard line.
- **Color is present but rationed.** The dashboard is mostly neutral; color
  appears only where it carries meaning:
  - **Status dots**: green = ready/good, red = error/critical, amber = warning —
    small filled dots beside the label, plus the label text in the same hue.
  - **A filled blue "Production/primary" pill** (with an up-arrow icon) vs. a
    neutral outline "Preview" pill. Blue is the accent: active nav, links,
    primary emphasis.
  - **Amber "Needs Attention" pills** — dark amber fill, amber text — for
    warnings. Maps directly onto our safety `warning` severity.
  - Charts use **categorical line colors** (blue / orange / green) on a very
    faint grid with an emphasized just-now endpoint.
- **Buttons have a clear hierarchy.** The one primary action is a **high-contrast
  solid** (white on dark, e.g. "Buy"); secondaries are **dark outline** pills
  ("Connect External", "Transfer In"). Matches our one-primary-action rule.
- **Type hierarchy is crisp.** Large bold near-white page titles; a muted grey
  subtitle line; tiny **uppercase letter-spaced section labels** in the sidebar
  (`COMPUTE`, `CDN`, `SERVICES`) separating grouped items; monospace for hashes
  and identifiers.
- **Density and rhythm.** Generous row height, hairline dividers between rows,
  right-aligned metadata + avatar, consistent medium radius (~8px) on every
  pill / input / card / button. Cards (Observability) are a hair lighter than the
  ground, rounded, with a title + big number + chart and a chevron affordance.
- **Inputs**: dark fill, subtle border, rounded, with a right-aligned keyboard
  hint chip (`/`, `F`). Filter dropdowns are equal-height dark pills with a
  chevron.

### Translate that into our tokens

- **Add a brand accent hue** — a refined Vercel-style blue — on `--primary`,
  `--accent`, `--ring`, and interactive/hover/focus states, tuned for both
  themes.
- **Add a semantic palette** — `good` / `warning` / `critical` / `info` tokens,
  wired to the vocabulary already in `ui/src/lib/labels.ts` (`SEVERITY_LABELS`,
  `VERDICT_LABELS`, `STATE_BADGES`), so the safety/drift badges read by color at
  a glance (green/amber/red/blue dots + pills), exactly like Vercel's status
  column.
- **Deepen and cool the neutrals** — a true near-black dark ground and
  slightly-cool-biased greys, not pure `0`-chroma; hairline borders at low
  opacity.
- Keep **light / dark / system** — the three-state theme is wired in `index.css`
  (`:root` = light, `.dark` = dark, `@theme inline` maps tokens to Tailwind).
  Redo the palette in all three consistently; check contrast. Vercel's reference
  is dark, but light must stay first-class.

### Where the work lives

- **`ui/src/index.css`** — the token source of truth and the highest-leverage
  file. **All color changes go here.** `tools/guard` BANS raw hex/rgb/hsl in
  `.tsx` (line ~83) — color lives in the stylesheet as tokens only.
- `ui/src/components/ui/*.tsx` — the 16 shadcn primitives. Audit each for a
  consistent radius / border / focus ring / hover / disabled / size scale.
  These are guard-exempt from the line cap (generated), but make them coherent.
- Pages: `overview.tsx` (triage Home), `review.tsx`, `library.tsx` +
  `components/library/*`, `tools.tsx` + `components/tools/*`, `customize.tsx`,
  `settings.tsx`, `sidebar.tsx`, `components/page-header.tsx`,
  `components/safety-findings.tsx`, `components/sync-scope.tsx`.

### Method — drive the real app in a browser and compare against Vercel

**This is not a fire-and-forget edit. Load the app in a browser and actively,
repeatedly compare each screen side-by-side against the Vercel references, the
way a designer would.** Concretely, on every iteration:

1. **Run the front end in a browser you can drive.** The app is a Tauri front end
   (React), so use the mock which serves it over plain HTTP for Chrome:
   `cd ui && VITE_MOCK=1 npx vite --port 5273`, then drive it with the
   **agent-browser** skill (Chrome via CDP). The mock swaps the Tauri bridge for
   in-memory data (`src/dev/mock*.ts`, `src/dev/fixture-declared.ts`) and already
   includes a blocked-safety item so Home's "held back for safety" row and the
   Review page's findings render with real color. (The real Tauri window itself
   is WebKit and hard to drive headless — do the visual iteration against the
   Vite mock in Chrome; save the real-WebKit pass for Task 2.)
2. **Screenshot every page** to a dir **outside the repo** (guard has a 200 KB
   file ceiling — keep images out of the tree), e.g.
   `/var/tmp/.../scratchpad/ui-shots/`.
3. **Open your screenshot and the matching Vercel reference at
   `/home/method/dev/.vstack-design-refs/` side by side and name the specific
   gaps** — ground too grey? borders too hard? no color on the status badges?
   inconsistent radius/row-height? weak type hierarchy? primary button not
   distinct? Fix the token or component, re-run, re-compare. **Loop until the gap
   to Vercel is closed, not until the build passes.** The owner is judging
   against these exact screens.
4. Walk **every** page this way — Home, Review & apply, Library (Installed +
   Add), Tools & Projects, Customize, Settings — plus the shadcn primitives that
   appear on them (badge, button, card, input, select, tabs, dialog, table).
- Keep the guarantees intact: vocabulary stays in `labels.ts` (no eng-speak —
  already audited clean); exactly one primary action per view; confirm-with-
  preview before any file change.

### Done when

A considered palette with a real accent and semantic colors in both themes, all
through tokens; every page and shadcn component visually consistent; it reads
like Vercel; `tools/guard` green. Screenshots of every page, before/after.

---

## Task 2: real-app (Tauri) walkthrough

The mock loop exercises pages, not the product. Before sign-off, walk the real
Tauri app through the flows mocks can't: **preview → apply → rollback**, the
**v0.1 → v0.2 migration**, and **scope-busy / error** states.

- Launch: `cd crates/app && ../../ui/node_modules/.bin/tauri dev` (needs WebKit;
  WebKit-on-Wayland env quirks are documented in `crates/app/src/lib.rs`). The
  `/run` skill may help.
- These flows are already covered by the Rust integration suite
  (`crates/core/tests/{invariants,migration,byte_faithful,...}.rs`); this
  confirms them in the real window. If the app can't be driven headless in the
  environment, record that and rely on the test coverage plus a manual owner
  pass.

---

## Task 3: cut the v0.2.0 release (Phase 7) — owner-gated

The repo is release-ready: `ARCHITECTURE.md` + `docs/adapters/` carry every
durable decision; `CHANGELOG.md` Unreleased holds all twelve breaking-change
rows (B1–B12, cross-checked against the register in `PLAN.md`).

Per `docs/RELEASING.md` "User-supplied gates" and the release-state memory,
these are **the owner's** to trigger: updater signing keys (`TAURI_SIGNING_*`
repo secrets), publishing the draft, and real-repo migration. Do not cut the
release without them.

The mechanical steps (also in `.claude/skills/app-deploy/SKILL.md`):

1. Bump the workspace `version` in `Cargo.toml` **and**
   `crates/app/tauri.conf.json` to `0.2.0` — both must equal the tag minus `v`,
   or the update feed no-ops or loops.
2. Move `CHANGELOG.md` `## [Unreleased]` → `## [0.2.0] - <date>`; confirm every
   entry keeps its **Breaking** call-out and migration note.
3. Commit, tag `v0.2.0`, push the tag; CI builds each target and publishes a
   **draft** GitHub Release (CLI binaries, app bundles, `feed.json`); review,
   then publish.
4. **The closing commit deletes `docs/PLAN.md`, `docs/research/`, and the
   `ARCHITECTURE.md` pointer to `PLAN.md`** — do this AS the release, not
   before. Their durable content is already mirrored into `ARCHITECTURE.md`,
   `docs/adapters/`, and `CHANGELOG.md`.

Delete this handoff file in that same closing commit.

---

## File map

| Path | What |
|---|---|
| `docs/PLAN.md` | Remaining-work anchor (progress block at top) |
| `docs/ARCHITECTURE.md` | Invariants + durable decisions |
| `docs/adapters/` | Per-harness reference (survives research deletion) |
| `docs/RELEASING.md` | Release mechanics + user gates |
| `.claude/skills/app-deploy/SKILL.md` | Release step list |
| **`ui/src/index.css`** | **Design tokens — the palette to redo (Task 1)** |
| **`/home/method/dev/.vstack-design-refs/`** | **Owner's four Vercel reference screenshots (Task 1 target; outside the repo)** |
| `ui/src/lib/labels.ts` | User-facing vocabulary + severity/verdict maps |
| `ui/src/dev/` | Mock data for the `VITE_MOCK=1` browser loop |
| `tools/guard` | The gate — read it |
| `CHANGELOG.md` | Keep-a-changelog, hook-enforced |
