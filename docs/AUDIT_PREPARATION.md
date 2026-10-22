# External Audit Preparation

Tracks the roadmap item "External security audit before any mainnet consideration" (#130).
Selecting and paying a firm, and mainnet deployment, are maintainer decisions and out of scope here.

## Token transfer check-effects-interactions audit (#317)

Soroban invokes a token contract synchronously and rolls back the invoking transaction when an unhandled invocation fails. Nevertheless, each DAO path should validate its inputs, write its accounting effects, then call the external token contract. The table tracks every direct token-client `transfer(` in the non-test DAO source.

| Function | Direction | State before transfer | State after transfer |
| --- | --- | --- | --- |
| `membership::register_member` | Member → DAO | Membership record, counters and yield snapshot | Event |
| `membership::exit_dao` | DAO → member | Stake reset, member deactivation, contribution totals, yield snapshot | Event |
| `membership::claim_rewards` | DAO → member | Yield snapshot advanced | Event |
| `loans::disburse_approved_loan` | DAO → borrower | Loan and borrower accounting | Event |
| `loans::repay_loan_internal` | Borrower → DAO | Repayment totals, loan/member status and yield accounting | Event |
| `treasury::execute` | DAO → destination | Proposal status persisted as Executed | Event and idempotent status rewrite in wrapper |
| `staking::stake` | Member → DAO | Staked balance, global total, stake time | Event |
| `staking::unstake` | DAO → member | Staked balance, global total | Event |

Any newly introduced token transfer must be added to this table. The intended rule applies to both inbound and outbound transfers; validations should finish before accounting changes and no fallible accounting writes should be left after the interaction.

## 1. Audit scope

**In scope:** the single `dao` contract, `contracts/dao/src/` (non-test code, ~1,900 lines):

| Module | Concern |
| --- | --- |
| `lib.rs`, `admin.rs`, `registry.rs` | entrypoints, admin set, initialization |
| `membership.rs`, `staking.rs` | join/exit, exit-share math, staking weight |
| `treasury.rs`, `loans.rs` | treasury proposals and withdrawals, loan lifecycle, interest split, defaults |
| `privacy.rs` | commit/reveal voting |
| `storage.rs`, `types.rs`, `error.rs`, `util.rs` | persistence, TTL handling, ABI types, error codes |

**Out of scope:** the test suite itself, deploy scripts, the token contract (treated as an assumption below).

**Assumptions the auditor should confirm or challenge**

- The DAO token is a standard Soroban token contract that does not reenter and does not charge transfer fees (see #115, #116).
- The admin set is a trusted, small group; the contract is immutable after deployment (see `docs/MIGRATION.md`).
- Ledger time and sequence numbers are honest.

**Threat model:** does not exist yet and is tracked separately in #133. It is the first thing an auditor will ask for and should land before engagement.

## 2. Preparation checklist

- [ ] `main` compiles (#65) and CI is green.
- [ ] `cargo fmt --all --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` and the wasm build all pass.
- [ ] Every open security finding below is marked **fixed** or **accepted (with written reason)** by a maintainer.
- [ ] Threat model written (#133).
- [ ] ABI frozen (section 4).
- [ ] `docs/INVARIANTS.md` has no untested invariant without a tracking issue.

## 3. Open security findings to triage

The maintainers decide fix vs. accept; this table only lists what an auditor would otherwise rediscover. Snapshot of open `security`-labelled issues; refresh before engaging.

| Area | Issues |
| --- | --- |
| Authorization / initialization | #66 (`initialize` has no auth), #42 (duplicate admins can zero the admin set), #115 (token address not validated) |
| Governance | #67 (one admin can remove all others), #127 (1 bp consensus threshold), #44 (quorum headcount vs. stake-weighted votes) |
| Economics / treasury | #119 (no withdrawal size cap), #118 (proposal can pay its proposer), #70 (retroactive policy changes), #10 (`edit_loan_proposal` bypasses the loan-to-treasury cap) |
| Correctness / safety | #116 (external transfers before state commit), #81 (getters never extend TTL) |
| Process / supply chain | #103 (deploy script can hit mainnet), #100 (no licence policy) |

For each: record the outcome (`fixed in #PR` or `accepted: reason`) in a "Decision" column here when it is made.

## 4. ABI freeze

Post-audit changes invalidate findings and the contract cannot be upgraded afterwards, so:

1. Pick the audited commit and tag it `audit-candidate`.
2. From the tag until the report is delivered, only changes that are pure bug fixes requested by the auditor may touch `contracts/dao/src/`; new entrypoints, changed signatures and changed error codes are not allowed.
3. Any exception is recorded in this file with the reason and the auditor's acknowledgement.

## 5. Report and remediation

- Store the final report under `docs/audits/` in this repository.
- Open one issue per finding, labelled `audit`, linked from the report, and close the loop by listing the fixing PR next to each finding.
