# Security Policy

`pulsar-core-contract` is an on-chain Soroban smart contract holding
bridged deposit records. A vulnerability here can mean loss of funds or
loss of control over the contract (see `THREAT_MODEL.md`) — please report
privately, not through a public GitHub issue.

## Reporting a vulnerability

Open a [GitHub Security Advisory](../../security/advisories/new) on this
repository ("Report a vulnerability" under the Security tab). This reaches
maintainers privately and lets us coordinate a fix and disclosure timeline
before any public details are posted.

Include, as far as you're able to:
- The affected entry point(s) or code path.
- Preconditions (contract state, which role — admin, relay signer, or
  neither — is required).
- A concrete scenario demonstrating impact — funds at risk, unauthorized
  state transition, admin/relay-signer role compromise, or a way to bypass
  a documented guard (the state machine, the upgrade timelock, the
  idempotency checks — see `THREAT_MODEL.md` for what's already
  considered).

## Scope

In scope: `src/`, and the deployment/upgrade process described in
`DEPLOYMENT.md`. Out of scope: the off-chain `pulsar-core` relay and
`pulsar-web` frontend (sibling repos — report there instead), and findings
already tracked as accepted risk with a documented rationale in
`THREAT_MODEL.md` (still worth reporting if you believe the accepted-risk
reasoning itself is wrong, just not as a novel finding).

## What to expect

This is a small project without a dedicated security team or a bounty
program. We'll acknowledge reports as soon as we can and work with you on
a disclosure timeline once a fix is available — please don't publicly
disclose before then.
