<!-- buNtu pull request template. Keep it short: one pull request, one task. -->

## Task and requirements

<!-- Task ID (T0.2, T1.4…) and the FR-*/NFR-* requirements it covers. -->

## What changed

<!-- Summary, files touched, and what was deliberately left out. -->

## Verification

<!-- Exact commands run and their result; paste the output. -->

## Checklist

- [ ] Scope limited to the task's listed files
- [ ] New or changed behaviour covered by a test that failed before and passes now
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo nextest run`, `cargo deny check` pass (from T0.2)
- [ ] No secret, key or personal data added
- [ ] Documentation updated when behaviour or configuration changed
- [ ] Commits signed off (`git commit -s`, DCO)
- [ ] One pull request, one task: the task's acceptance criterion is demonstrated above
