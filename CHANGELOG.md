# Changelog

All notable changes to `ourdao-contracts` are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project aims to follow [Semantic Versioning](https://semver.org/). The workspace version in `Cargo.toml` has been `0.1.0` since the first commit; entries below are grouped under **Unreleased** until a version is cut, at which point the version in `Cargo.toml` should be bumped alongside a dated heading here.

## [Unreleased]

### Changed
- `exit_dao` / `calculate_exit_share` are now O(1): a maintained `ActiveContributions` total replaces the per-member scan (#353).
- `treasury::tally` and `loans::vote_on_loan_proposal` read the active-member count (and policy/threshold) once per call (#355).
- `bench-cost.sh` now runs a real `test_budget_scaling` benchmark that also covers `request_loan_in_asset`, `repay_loan_in_asset` and `propose_withdrawal_in_asset` (#354).

### Fixed
- Added regression coverage that `reveal_vote` removes the stored commitment (#356).

### Added
- Multi-token treasury (#236): `request_loan_in_asset`, `repay_loan_in_asset`, `propose_withdrawal_in_asset`, `get_treasury_balance_in_asset`, and the token whitelist (`propose_token_addition` / `execute_token_addition` behind the 48h timelock, `remove_token`, `is_token_whitelisted`, `get_whitelisted_tokens`). The base token is always whitelisted. New append-only error codes 83–87 and events `loan_asst`, `tre_asset`, `tok_prop`, `tok_add`, `tok_rem`.
- `cancel_loan_proposal` and `cancel_treasury_proposal` entrypoints for the existing #185 cancel logic.
- CI: a fast `check` job (`cargo check --locked --all-targets`) that `build-and-test` and `reproducible-build` depend on, so a broken merge fails within minutes (#238).
- Regression coverage ensuring `get_version` stays aligned with the Cargo package version (#372).
- `repay_loan_partial` for partial loan repayment.
- Loan and proposal lifecycle expiry, a `has_voted` view, name validation and an O(1) pull-based yield accumulator.
- `expire_treasury_proposal`, exposed in `lib.rs`; treasury proposals now expire after `VOTING_PERIOD`.
- Commit-reveal phase guards for private proposals (#57).
- `NotEditingPhase` and `DocumentTooLarge` errors; document hashes are limited to 64 bytes (#58, #59).
- `YieldRemainder` storage: `distribute_interest` carries rounding remainders and emits an event (#60).
- Loan terms are re-quoted at disbursement (#61).
- `bench-cost.sh` and a budget scaling benchmark; resource costs and the membership ceiling documented (#138).
- `generate-abi.sh` for a wasm-derived interface (#135).
- Property tests for exit-share, interest and staking-weight math.
- `util::isqrt`: an overflow-free integer square root over the whole `i128` range, backing the quadratic staking curve (#182).
- CI: `cargo clippy`, `cargo audit` and a `cargo-llvm-cov` coverage floor of 95% (#62).
- CI: MSRV job tests minimum supported Rust version (1.84) across all targets (#381).
- CI: markdown link checker validates all `.md` files for broken references (#383).
- PR template: checkboxes for "single issue only" and "ABI/event changes" (#382).

### Changed
- `request_loan` and `propose_treasury_withdrawal` now delegate to their `_in_asset` versions with the base token; behaviour and events are unchanged.
- `Member.join_ledger` renamed to `join_time` (#55).
- Staking boost is quadratic instead of linear: `voting_weight` is now `1 + min(isqrt(stake / 100), 5)`, so the *k*-th bonus vote costs `k² × 100` staked tokens and a whale can no longer buy proportional control (#182).
- Admin policy bounds tightened (#53, #54); loan eligibility checks and the `is_eligible_for_loan` signature corrected (#52, #56).
- Proposal document attachment is restricted to the proposal's own owner, and rejected after the editing phase (#59).

### Fixed
- `main` compiles again after the corrupted merge of #216 (#235); `cargo fmt` and `cargo clippy -D warnings` pass again.
- `total_members` is now decremented on exit, so exit shares no longer underpay (#11).
- Stake locking and `due_time` on loan approval (#46, #51); reveal period end (#45).
- CI workflow and formatting violations.

### Documentation
- Removed the orphaned, never-compiled `src/test.rs`; its four unique tests were ported to `src/tests/` (#237).
- `SECURITY.md` with a coordinated disclosure policy.
- `docs/MIGRATION.md`, `docs/INVARIANTS.md`, `docs/LOAN_POLICY_GUIDE.md` and the pause/unpause incident runbook.
- `docs/THREAT_MODEL.md`: actors, trust assumptions and known gaps (#133).
