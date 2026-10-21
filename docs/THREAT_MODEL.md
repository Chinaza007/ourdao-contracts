# Threat Model and Trust Assumptions

Status: testnet-stage, **not externally audited**. This document states what the contract assumes and what it does not defend against. It complements [`SECURITY.md`](../SECURITY.md) (reporting) and [`docs/INVARIANTS.md`](./INVARIANTS.md) (what must always hold).

## Assets

- Treasury funds held by the contract in the DAO token.
- Member stakes and accrued yield.
- Governance state: proposals, votes, loans, membership records.

## Actors and trust

| Actor | Trusted to | Not trusted to |
| --- | --- | --- |
| Admin set | Perform admin-only entrypoints (policy, pause/unpause, migration seeding) and stay online | Move member funds outside the entrypoints' rules |
| Members | Vote and act only for their own address (`require_auth` per call) | Act on another address's behalf |
| Anyone | Call permissionless entrypoints such as `mark_loan_defaulted`, which is a pure function of ledger time and loan state | Anything else |
| DAO token contract | Behave as a standard Soroban token | Nothing beyond that; a malicious token is out of scope, the DAO token is chosen at initialization |
| Off-chain backend and frontend | Index and display state | They are not a source of truth; the contract is |

## Trust assumptions

1. **Admins are honest and available.** Admin keys can pause the contract and set policy within bounds. A compromised admin quorum is the largest single risk. The consensus threshold is set at initialization.
2. **The DAO token is well behaved.** Balance-changing operations follow checks-effects-interactions, but a token that lies about balances or transfers breaks accounting.
3. **Migration seeding is trusted.** A migrated deployment relies on admins seeding state truthfully; see [`docs/MIGRATION.md`](./MIGRATION.md#trust-assumption).
4. **The contract is immutable.** There is no upgrade path, so a bug means migration to a fresh deployment.
5. **Ledger time is the only clock.** Voting periods, expiry and defaults use ledger timestamps.

## Threats considered

| Threat | Mitigation |
| --- | --- |
| Acting on another member's behalf | `require_auth()` on the member address in every state-changing entrypoint |
| Double or miscounted votes | Per-member vote tracking (`has_voted`), proposal phase guards, voting-period expiry |
| Private proposal stalling or reveal manipulation | Commit-reveal phase guards; see the open reveal-deadline and membership-fee issues below |
| Reentrancy | Soroban has no mid-call re-entry; state is still updated before token transfers |
| Rounding drift in yield | `YieldRemainder` carried forward, property tests |
| Unbounded storage / cost growth | Document hash size cap, documented membership ceiling and resource costs (#138) |
| Sybil / vote buying via cheap memberships | See [Sybil and Cheap Membership Attacks](#sybil-and-cheap-membership-attacks) below |

## Sybil and Cheap Membership Attacks

### Attack Mechanics

Every member in the DAO receives a base voting weight of 1 upon paying the one-time `membership_fee` ($F$). When $F$ is set too low relative to treasury assets or available loan sizes, an attacker can register multiple Sybil accounts to purchase voting power and push through malicious loan proposals or treasury withdrawals.

### Cost-to-Attack Formula

Let:
- $M$ = number of existing honest active members
- $S$ = number of Sybil accounts created by the attacker
- $T$ = consensus/quorum threshold in basis points (e.g., $5\,100$ for 51%, $6\,000$ for 60%)
- $F$ = `membership_fee` (in base DAO tokens)
- $W$ = amount extracted via malicious loan or treasury withdrawal

To pass a proposal over honest opposition (all $M$ honest members voting against), the attacker needs their Sybil votes to satisfy:
$$\frac{S}{M + S} \ge \frac{T}{10\,000} \implies S \ge \left\lceil M \cdot \frac{T}{10\,000 - T} \right\rceil$$

For example, at a 51% threshold ($T = 5\,100$), $S \ge \lceil M \cdot \frac{5\,100}{4\,900} \rceil \approx 1.041 \times M$ accounts.

The upfront capital cost to execute the attack is:
$$\text{Cost}_{\text{attack}} = S \times F$$

The attack is economically rational whenever the extractable value exceeds the cost:
$$\text{Net Profit} = W - (S \times F) > 0$$

If $F$ is nominal (e.g., 1 token) and treasury balance or loan cap is large, $\text{Cost}_{\text{attack}} \ll W$, allowing trivial drained funds.

### Recommended Mitigations and Policy Ranges

1. **Adequate Membership Fee ($F$)**:
   - $F$ must scale with the DAO's treasury size.
   - **Recommended range**: Set $F$ such that acquiring $S$ accounts exceeds the maximum single loan limit ($W_{\max} = \text{Treasury} \times \text{max\_loan\_to\_treasury\_ratio}$). Specifically, ensure $F \ge \frac{W_{\max}}{S_{\min}}$.
2. **Minimum Membership Duration (`min_membership_duration`)**:
   - Prevent flash-joining to vote on active proposals.
   - **Recommended range**: $7 \text{ to } 30 \text{ days}$ ($604\,800$ to $2\,592\,000$ seconds). New members must wait this duration before voting or requesting loans.
3. **Voting Quorum (`quorum_bps`)**:
   - Set quorum thresholds high enough to prevent an attacker from exploiting low honest participation.
   - **Recommended range**: $5\,000 \text{ to } 6\,667 \text{ bps}$ (50%–66.67%).
4. **Staking Weight Incentives**:
   - Rely on stake-weighted voting (`util::voting_weight`) to dilute flat 1-member-1-vote power by rewarding long-term capital lockup.

## Known gaps

- No external audit.
- Fixed membership fee and no reveal deadline on private proposals are tracked as separate issues (#47, #48).
- Testnet-only deploy tooling.
- Admin-key custody and rotation are operational concerns outside the contract.
