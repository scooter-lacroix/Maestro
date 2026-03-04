# TrackLens Full Port — Risk Log

- **Scope creep across platforms**  
  - Likelihood: Medium | Impact: High  
  - Mitigation: Freeze scope to FR-1..FR-10; reject new features unless logged as follow-up track.

- **Browser dependency (default-on)**  
  - Likelihood: Medium | Impact: Medium  
  - Mitigation: Confirm browser availability in environments; retain minimal text fallback path; document toggle.

- **Rebranding gaps (plannotator remnants)**  
  - Likelihood: Low | Impact: High  
  - Mitigation: Mandatory grep audit before rollout; block release on any match.

- **Cross-platform parity drift (Claude vs OpenCode vs Pi)**  
  - Likelihood: Medium | Impact: High  
  - Mitigation: Shared test checklist per platform; align tool names/params; smoke tests per Phase 4–6.

- **UI bundle size/performance**  
  - Likelihood: Medium | Impact: Medium  
  - Mitigation: Single-file Vite bundle with tree-shake; measure load time; defer heavy assets.

- **Rust/Node divergence**  
  - Likelihood: Medium | Impact: Medium  
  - Mitigation: Mirror types (TS ↔ Rust) and regenerate when changing schemas; cargo tests gating.

- **Walkthrough diff accuracy**  
  - Likelihood: Medium | Impact: Medium  
  - Mitigation: Validate changed-file detection against git refs; include snippets and line ranges; add tests.

- **DB/file permission issues (memory store, bundles)**  
  - Likelihood: Low | Impact: Medium  
  - Mitigation: Verify writable paths early; fallback to CLI memory store with elevated perms if needed.

- **Manual E2E reliance**  
  - Likelihood: Medium | Impact: Medium  
  - Mitigation: Script smoke tests where possible (hook stdin, tool calls); keep E2E checklist in plan Phase 9.

- **Toggle misuse (disabled unintentionally)**  
  - Likelihood: Low | Impact: Medium  
  - Mitigation: Default-on check in rollout; log current toggle state in docs; surface status in CLI output.
