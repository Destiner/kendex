# Phase 4 review: Community directory + skills.sh (app side, ungated slice)

You are reviewing kendex-app at /home/method/dev/vstack2 — commits 75d039e and 2e080cf (the phase-4 slice). Read the diff (`git show 75d039e`) and the files in full:

- crates/core/src/registry.rs (Fetch trait, CurlFetch, curl -i response parsing)
- crates/core/src/registry/index.rs (strict parse + caps of the kendex.ai schema-1 index)
- crates/core/src/registry/cache.rs (ETag/TTL/offline ladder, on-disk cache)
- crates/core/src/registry/skillssh.rs (versioned search adapter, kill switch)
- crates/core/src/registry/view.rs (subscribed-state merge)
- crates/core/src/repo_move.rs (new pub owner_repo)
- crates/core/tests/registry.rs
- crates/app/src/community.rs
- ui/src/stores/community.ts, ui/src/components/marketplaces/community-tab.tsx, ui/src/components/marketplaces/skillssh-search.tsx, ui/src/dev/mock-community.ts
- docs/ARCHITECTURE.md registry seam bullet (accuracy vs code)

Context you may trust: the plan (docs/plans/marketplaces.md §3.2 Community tab, §5.7 skills.sh) demands: index parses with caps and refuses malformed; ETag/TTL/offline; subscribe from a row; skills.sh result → subscribe + install; adapter has strict schema, caps, kill switch; a hit is a lead never an identity; the tab never blank offline. Sign-in/collections/deep links are deliberately deferred to W3/W4.

Lenses, priority order:
1. **Adversarial**: a hostile or compromised kendex.ai response (or MITM'd skills.sh) — can it inject through curl args, oversize the cache, smuggle a repo string that subscribe treats differently than the directory showed, or wedge the tab? curl -i parsing edge cases (folded headers, 1xx interim, huge bodies, non-UTF8). The cache files on disk as an attack surface.
2. **Correctness vs plan**: TTL/ETag/stale ladder holes (e.g. 304 with no cache, meta/body divergence after a crash between the two writes), subscribed-merge misses (URL spellings), skills.sh URL construction vs what source_ops::subscribe accepts.
3. **UX/taste** (memory: concrete copy, no floating text, designed states): loading state, error states, the stale line, empty states, the sub-tab toggle.
4. **Test adequacy**: which of the plan's pinning tests are missing or weak.

Rules: concrete findings only — file, line, failing scenario, severity high/medium/low. Check before writing: if handled, don't report it. Write findings to /home/method/dev/vstack2/docs/reviews/pi-phase4-review-findings.md (NOT any earlier phase file, NOT the w2 file). End with a line containing exactly: REVIEW-COMPLETE
