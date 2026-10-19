# ADR-0005: Interest discount from member loan history

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

`quote_terms` prices every loan from its size relative to the treasury
alone:

```text
rate = min_interest_rate + loan_ratio * (max_interest_rate - min_interest_rate)
```

A member who has repaid five loans pays the same as one borrowing for the
first time. `get_member_loan_stats` (#190) already exposes each member's
lifetime `total_loans`, `repaid_loans` and `active_loans` as an O(1) read
of the `Member` record, and those counters survive exit and rejoin. That is
an on-chain repayment history the rate could reward. Repayment history is
the main input of both traditional and on-chain credit scores.

The constraints:

- **Sybil resistance.** Anyone can make a new address, so history can only
  ever *lower* a rate. A fresh address must never do better than today.
- **Members' floor.** `min_interest_rate` is the yield members are
  guaranteed on any loan. A discount must not go below it.
- **Storage compatibility.** `LoanPolicy` and `Member` are stored structs;
  new fields would break decoding of existing entries (see ADR-0004).
- **Signal quality.** The counters record no timing or size: a loan repaid
  late within the grace period, or a very small loan, counts the same as
  any other.

## Decision

Apply a **reputation discount to the premium above `min_interest_rate`**,
computed from `MemberLoanStats` whenever a loan is quoted for a specific
borrower.

### Score

```text
defaults = total_loans - repaid_loans - active_loans   (saturating)
score    = clamp(repaid_loans - DEFAULT_WEIGHT * defaults, 0, MAX_SCORE)
```

with `DEFAULT_WEIGHT = 3` (one default cancels three repayments) and
`MAX_SCORE = 5`.

### Rate

```text
premium       = rate - min_interest_rate        (from quote_terms, unchanged)
discount      = floor(premium * score * STEP_BPS / BASIS_POINTS)
borrower_rate = rate - discount
```

with `STEP_BPS = 1_000`, so each point of score removes 10% of the premium
and the maximum discount is half of it. The discount is rounded down, so
rounding favours members, and `borrower_rate` can never fall below
`min_interest_rate`.

Worked example with `min 5%`, `max 20%` and a loan of 25% of the treasury
(undiscounted rate 8.75%):

| Score | 0 | 1 | 2 | 3 | 4 | 5 |
| --- | --- | --- | --- | --- | --- | --- |
| Rate (bps) | 875 | 838 | 800 | 763 | 725 | 688 |

### Where it applies

- `request_loan` / `request_loan_in_asset` store the discounted rate on the
  proposal, so voters see the rate the borrower would actually pay.
- The disbursement re-quote (#61) recomputes the discount from the
  borrower's stats at that moment, just as it already recomputes the size
  premium from the current treasury. The two usually match, but they can
  differ if the borrower had another proposal disbursed and then repaid or
  defaulted in between. Using current history at disbursement is the
  consistent choice.
- `calculate_loan_terms(amount)` is left as it is: the undiscounted,
  borrower-agnostic quote.

### ABI additions

| Item | Purpose |
| --- | --- |
| `calculate_loan_terms_for(member, amount) -> LoanTerms` | Quote including that member's discount. |
| `get_reputation_discount(member) -> u32` | Current discount as basis points of the premium (0 to 5,000). |
| Event `rep_disc`: `(loan_id, borrower, score, discount)` | Emitted at disbursement when `discount > 0`; `loan_appr` is unchanged. |

No storage, `LoanPolicy` or `Member` changes, and no new errors. The
constants are compiled in; tuning them later would mean a separate
`ReputationPolicy` key with these values as defaults.

### Gaming analysis

- **New addresses** score 0 and pay today's rate, so a Sybil gains nothing.
  The score belongs to one address and can't be transferred.
- **Exit and rejoin** keeps the lifetime counters, so a default can't be
  laundered by leaving and coming back.
- **Farming with tiny loans:** each loan still needs a member vote, the
  `cooldown_period` between loans and `min_membership_duration`. The payoff
  is capped at half the premium, and small loans carry almost no premium,
  so the discount matters on exactly the loans members scrutinize most.
  Members can check `get_member_loan_stats` before voting.
- **Late or extended repayments** count as repaid, because the counters
  carry no timing. A loan extended under ADR-0004 that is then repaid is
  treated as a success. This is the main weakness of the signal; see
  Consequences.

## Alternatives considered

- **A flat discount in bps off the rate.** Can drop below
  `min_interest_rate` and erode members' guaranteed yield on small loans.
  Rejected.
- **A time-decayed score.** Needs per-loan timestamps that `Member` doesn't
  hold and that couldn't be added without a storage migration. Deferred; the
  `loan_rpy` / `loan_dflt` events already let an indexer compute it.
- **An off-chain credit-score oracle.** Adds a trusted party and an
  integration for data the contract already has. Rejected.
- **A rebate paid at repayment** instead of a lower quoted rate. Same
  economics but more state and a second transfer. The early-repayment
  discount (#363, ADR-0008) is the place for payoff-time incentives, and it
  composes with this one: this ADR changes the quoted rate, ADR-0008 the
  amount due at payoff.
- **Excluding any member with a default, permanently.** Simpler, but gives
  no route back. Weighting a default as three repayments lets a member
  rebuild history.

## Consequences

- Reliable borrowers pay up to half the size-based premium less, which
  makes borrowing from the DAO more attractive to the members least likely
  to default. Members give up the same amount of yield on those loans.
- Quotes now depend on the borrower, so clients must call
  `calculate_loan_terms_for` to show the real rate. The existing view stays
  correct as the "no history" price.
- No extra ledger reads in the request path: the borrower's `Member` record
  is already loaded by `require_active_member`.
- The signal ignores lateness and loan size. A follow-up could count only
  on-time repayments above a minimum principal in a separate per-member
  counter, keeping `Member` unchanged.
- **Implementation tests:** the score formula including saturation and
  clamping; the floor at `min_interest_rate`; the worked example above;
  that request and disbursement quotes agree; that the `rep_disc` event
  appears only when `discount > 0`; and that history survives exit and
  rejoin.
