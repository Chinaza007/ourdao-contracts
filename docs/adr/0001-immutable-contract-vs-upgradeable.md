# ADR-0001: Immutable contract vs upgradeable contract

- **Status:** Accepted
- **Date:** 2026-10-17

## Context

OurDAO holds member funds and exposes a public Soroban ABI. An upgrade hook
could repair a discovered bug without migrating balances, but it would also
allow a privileged party (or a compromised governance key) to replace the
rules that members approved. The current contract has no proxy, wasm upgrade
entrypoint, or hidden administrator bypass.

## Decision

The deployed OurDAO contract is immutable. A new implementation is released
as a new contract and adopted through an explicit migration process. The
release must include the optimized wasm checksum, deployment identifiers, and
any required state/export tooling. Governance configuration (admins,
thresholds, policy, and pause) may change through the existing authenticated
entrypoints, but it must not change the executable code.

## Alternatives considered

- **Admin-controlled wasm upgrade:** rejected because a single operational key
  could change fund-moving logic without member approval.
- **DAO-voted upgrade:** safer than a single key, but still expands the trust
  and audit surface and requires a migration-safe storage ABI.
- **Proxy/delegate contract:** rejected because cross-contract calls add
  complexity to authorization, storage, and incident recovery.

## Consequences

Immutability makes the deployed behavior auditable and removes upgrade-key
custody risk. A critical bug requires pausing the contract, releasing a new
version, and coordinating a migration; this is slower and may require members
to re-authorize the new contract. `DEPLOYMENTS.md` and release checksums are
therefore part of the operational source of truth. Any future upgrade design
must be proposed in a new ADR and include a storage migration, governance,
rollback, and independent audit plan.
