## What changed and why

<!-- Not just what the code does -- link the motivating issue/context. -->

## Checklist

- [ ] `make check` passes locally (fmt -> wasm build -> clippy -D warnings -> test).
- [ ] New/changed entry points: access control is the first thing the
      function does, inputs go through `validation.rs`, and both the
      happy path and the auth-failure path are covered by a test
      (`CONTRIBUTING.md`).
- [ ] If this changes an emitted event's shape or `EVENTS.md`'s version:
      `EVENTS.md`, `CHANGELOG.md`'s "Event schema" section, and sibling
      repos (per `CLAUDE.md`'s cross-repo coordination section) are all
      accounted for in this change, not left as a follow-up.
- [ ] If this closes, reopens, or introduces a `THREAT_MODEL.md` finding,
      that file is updated in this change.
- [ ] Non-obvious decisions have an ADR in `docs/adr/`.
