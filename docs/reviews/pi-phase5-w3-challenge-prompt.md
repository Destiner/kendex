# Adversarial plan challenge: phase 5 (authoring) + W3 (sign-in/submit) — before building

You are the cross-model challenger standing in for the scheduled Codex re-check. Target: the PLAN, not code. Read /home/method/dev/vstack2/docs/plans/marketplaces.md — focus on:

- §3.6 (Mine tab flows: use existing / create / import / check)
- Phase 5 row in §7 (deliverables + pinning tests)
- §5.4 W3 (sign-in, /submit ownership verification, /me, device flow, tokens, rate limits) — NOTE: auth library is now **BetterAuth** (owner decision 2026-08-19, replacing Auth.js everywhere; sessions live in Neon Postgres)
- Phase 6 row (publish + submit from the app) — it builds directly on W3, so W3 design flaws surface here
- §5.8/W4 collections only where W3 decisions constrain them

Also read docs/plans/marketplaces-tasks.md (Log + Process gates) so you know what already shipped (phases 0-4, W1, W2, W5) and don't re-litigate it.

Your job: break the plan before the build does. Priorities:

1. **W3 security/authority**: device-flow design (code entropy, polling, token storage, revocation), push-access-not-ownership claims, private-repo leak paths through /submit or /me, rate-limit gaps, BetterAuth-specific pitfalls (session/account model, device-flow support — BetterAuth has no first-party device-flow plugin: is the plan's hand-rolled device flow sound, or should it be hardened/replaced?), CSRF/redirect handling on kendex.ai.
2. **Phase 5 correctness**: use-existing with zero writes vs. later check/import steps that want to write; import licence confirmation edge cases; scaffold byte-stability claim; three import origins — what breaks with a marketplace-origin item that was edited locally?
3. **Sequencing/dependency traps**: anything in phase 5/6/W3 that assumes a thing phases 0-4/W2/W5 did NOT actually build.
4. **Underspecification** that would force mid-build guessing: name it and propose the decision.

Findings: concrete, severity critical/high/medium/low, each with the plan section it hits and a proposed plan amendment. No style notes. Check the plan text before claiming something is missing.

Write to /home/method/dev/vstack2/docs/reviews/pi-phase5-w3-challenge-findings.md (NOT any phase1-4 or w2 file). End with a line containing exactly: REVIEW-COMPLETE
