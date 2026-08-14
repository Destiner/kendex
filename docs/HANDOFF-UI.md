# vstack2 UI round 3 — handoff

Run this to completion in `/home/method/dev/vstack2`. Owner-approved scope, four
tasks. Read `docs/ARCHITECTURE.md` and `tools/guard` first (guard is the gate:
fmt/clippy/cargo test, tsc/biome/vitest, no raw colors in .tsx, 250-line caps,
changelog required for `crates/`/`ui/` commits). Standing rules: Sonnet-class
agents do UI implementation, the main loop reviews screenshots against the mock
— it does not pixel-push; engine/app-shell (Rust) changes get an adversarial
review; every behavior change ships with a test that fails without it.

**Method (all tasks):** drive the mock in Chrome — `cd ui && VITE_MOCK=1 npx
vite --port 5273`, agent-browser skill, screenshots to a dir OUTSIDE the repo.
The fixtures already mirror the owner's real machine density (7 hooks sharing
one finding, 21-plugin affected sets are the real numbers to design for — grow
fixtures if reality outpaces them). Check dark AND light. Owner judges against
Vercel's dashboard polish; copy stays in `ui/src/lib/labels.ts`, product words
only, no engine-speak, no numeric scores in rows.

## 1. Migrate Radix → Base UI (owner decision: "we should be using base")

Use the installed `migrate-radix-to-base` skill (`.claude/skills/`). Nine
primitives in `ui/src/components/ui/` import from the `radix-ui` package
(checkbox, dialog, label, scroll-area, select, separator, switch, tabs,
tooltip); the rest are plain elements. Migrate all, remove the `radix-ui` dep,
add the Base UI dep (one-line justification in the commit message per repo
rule), update `ui/components.json` accordingly. Done when: pixel parity with
before-screenshots on every page (our custom styling — underline tabs, compact
cards, tokens — must survive), keyboard behavior verified in-browser on Select,
Dialog, Tabs, Tooltip (open/close/arrow/escape/focus-trap), guard green.

## 2. shadcn-skill pass — methods, UI, code quality

Use the installed `shadcn` skill (project-aware; reads `ui/components.json`).
Sweep for: (a) primitives that drifted from current shadcn canon where upstream
is simply better — adopt improvements WITHOUT losing our deliberate design
decisions; (b) simpler composition patterns for our hand-rolled bits
(status-dot, stat-tile, confirm-dialog, page-header) if shadcn offers a cleaner
method; (c) code structure/simplicity in `ui/src` per the repo rule "prefer
deleting code to abstracting it" — dead variants, needless wrappers, duplicated
row layouts that should share one component. Report what you changed and what
you deliberately kept custom, and why.

## 3. Review & apply, round 3 — the affected-list walls

Owner: still the worst page. The grouped findings landed, but with real data
the "Affects 21 plugins:" identifier dump is a wall of grey mono text, printed
TWICE because two distinct findings affect the identical plugin set. Fix:

- Collapse affected lists: show the count + first ~4 identifiers + "+17 more"
  behind an expand affordance (disclosure/chevron; shadcn collapsible is fine).
- When multiple findings share the identical affected set, group by the SET:
  one "21 plugins from the Codex bundles" block with its findings stacked
  beneath, affected list shown once.
- Long identifier runs stay mono text-xs muted and break between identifiers
  (already done — keep it).
- While there: re-judge the whole page at density like a designer — spacing,
  hierarchy, what deserves to be above the fold.

## 4. Navigation: back / breadcrumb + frameless window

**Back/breadcrumb:** cross-page links now exist (stat tiles, count pills →
prefiltered Library, triage rows → Review). Add a slim bar at the top of the
content panel: a back button (chevron-left, appears only when there is history)
plus a breadcrumb of where you are ("Library / Installed", "Tools & Projects /
Projects"). Implement history as a small stack in `ui/src/stores/nav.ts`
(tested — push on cross-page navigation, pop on back; sidebar clicks reset, no
infinite growth). Keep it quiet: text-xs muted, no border weight.

**Frameless window with in-app titlebar:** remove the system title bar —
`"decorations": false` in `crates/app/tauri.conf.json` — and embed the bar in
the app: drag region across the top (`data-tauri-drag-region` attribute — plain
HTML, no import), double-click to maximize, and standard window controls
(minimize / maximize-restore / close) top-right as shadcn ghost icon buttons
matching both themes. Guard bans `@tauri-apps` imports outside generated
bindings — so add three thin commands in `crates/app` (minimize,
toggle-maximize, close on the focused window) exposed via tauri-specta so they
arrive through generated `ui/src/bindings.ts`. In `VITE_MOCK` the controls
render but no-op (mock the bindings like the rest). This is a `crates/app` +
`ui/` change: CHANGELOG entry required; run the adversarial review on the Rust
side; verify the real window via `cd crates/app && ../../ui/node_modules/.bin/tauri dev`
(drag, snap, double-click-maximize, controls — Wayland: the WebKit DMABUF
workaround self-applies). Linux is the target; note macOS traffic-light
placement as future work in the commit body, don't attempt it.

## Wrap-up

Before/after screenshots for every task; guard green; conventional commits with
changelog entries (Radix→Base and the titlebar are user-visible "Changed"
rows); update `docs/PLAN.md` progress block; delete this file in the final
commit of the round.
