# ADR-0004: Loan due-date extension proposals

- **Status:** Proposed
- **Date:** 2026-10-19

## Context

A borrower who can see they will miss `Loan.due_time` has one outcome
available today: after `due_time + default_grace_period` anyone calls
`mark_loan_defaulted`, the borrower loses `default_penalty_bps` of their
contribution, and the treasury may never recover the balance. Members have
no way to grant more time even when they would rather be repaid late than
not at all.

Microfinance experience is that rescheduling early and selectively costs
the lender some delayed interest but avoids larger write-offs [1, 2]. The
opposite risk, rescheduling the same loan again and again so that a loan
which will never be repaid never shows as a default, is known in banking as
*evergreening*. DeFi governance votes that changed a live loan's terms have
also been contentious, so here the borrower asks and the lenders decide.

The design is constrained by the deployed storage layout. `Loan`,
`LoanPolicy` and `Member` are stored as `#[contracttype]` structs; adding a
field changes their encoding and breaks decoding of entries already on the
ledger. Loans also reuse their proposal's id, and CONTRIBUTING forbids a
second id counter.

## Decision

Add a borrower-initiated **extension request** that active members vote on.
At most one request per loan, ever.

### Entrypoints

| Entrypoint | Auth | Effect |
| --- | --- | --- |
| `request_loan_extension(borrower, loan_id, extra_time) -> Result<(), Error>` | borrower | Opens the request and quotes its fee. |
| `vote_on_loan_extension(voter, loan_id, support) -> Result<(), Error>` | voter | Records a weighted vote and applies the extension once approved. |
| `get_loan_extension(loan_id) -> Option<LoanExtension>` | none | View of the request, with an elapsed vote reported as `Expired`. |

`has_voted` gains a `ProposalKind::LoanExtension` variant. Appending a
variant keeps existing values decodable because `#[contracttype]` enums
encode by variant name. `attach_document` rejects that kind with
`InvalidProposal`.

### State

A new `LoanExtension` record under `DataKey::LoanExtension(loan_id)`:

```rust
pub struct LoanExtension {
    pub loan_id: u32,
    pub extra_time: u64,
    pub fee: i128,
    pub created_at: u64,
    pub voting_period: u64, // copied from policy, like proposals do
    pub status: ProposalStatus, // Pending, Approved, Rejected or Expired
    pub for_votes: i128,
    pub against_votes: i128,
    pub votes_cast: u32,
}
```

Votes are recorded under `DataKey::ExtensionVoted(loan_id, Address)`. Both
keys are persistent and follow the usual TTL extension on read and write.
The loan id is the record's identity, so no counter is added and
`loan.id == proposal.id` is untouched. `Loan`, `LoanPolicy` and `Member`
keep their current shape.

### Rules

`request_loan_extension` requires all of:

1. Initialized, not paused; `borrower` authorizes and is an active member who
   is `loan.borrower`.
2. `loan.status == Active` and no `LoanExtension` record exists for the loan.
   A rejected or expired request still uses up the loan's single request.
3. `0 < extra_time <= policy.max_loan_duration`.
4. **Request deadline:** `now + policy.voting_period <= loan.due_time +
   policy.default_grace_period`. Every vote ends before the loan can be
   marked defaulted, so `mark_loan_defaulted` needs no change and a request
   can never be used to delay a default.

**Fee.** The loan's own rate, pro-rated for the extra time, on what is still
owed:

```text
outstanding = total_repayment - amount_repaid
fee = ceil(outstanding * interest_rate * extra_time / (BASIS_POINTS * (due_time - start_time)))
```

The fee is quoted in the request and **added to `total_repayment` on
approval**, not paid up front, because a borrower asking for time usually
cannot pay now. Repayments are split interest-first, so the fee reaches
members as yield like any other interest (base-token loans) or stays in the
treasury (other assets). Pricing extra time at the loan's own rate means
extending is never cheaper than having borrowed for longer in the first
place. Rounding up keeps a non-zero fee on small balances.

**Voting.** Same weight (`util::voting_weight`) and threshold (`quorum_bps`,
else the consensus threshold) as loan approval, over the active-member base
minus the borrower. The borrower may not vote on their own extension
(`NotAuthorized`). Early rejection uses the same "cannot reach the threshold
any more" rule as loan proposals. Voting closes at `created_at +
voting_period`; a request still `Pending` after that is reported as
`Expired` and nothing changes.

**On approval:** `loan.due_time += extra_time` and `loan.total_repayment +=
fee`, in one write. The default grace period then runs from the new due
time. `MemberLoanStats` is unchanged: an extended loan that is repaid counts
as repaid (see [ADR-0005](./0005-reputation-rate.md)).

### Events

Symbols are at most nine characters to fit `symbol_short!`:

| Topic | Data |
| --- | --- |
| `ext_req` | `(loan_id, borrower, extra_time, fee)` |
| `ext_vote` | `(loan_id, voter, support)` |
| `ext_appr` | `(loan_id, new_due_time, new_total_repayment)` |
| `ext_rej` | `(loan_id, for_votes, against_votes)` |

Expiry emits no event; indexers derive it from `created_at + voting_period`.

### Errors

Appended after the last code in use when this is implemented, never
renumbered:

- `ExtensionAlreadyRequested`: the loan has already used its one request.
- `ExtensionWindowClosed`: the request deadline has passed.
- `InvalidExtension`: `extra_time` is zero or longer than
  `max_loan_duration`.
- `ExtensionNotFound`: there is no request to vote on.

Existing errors cover the other cases: `LoanNotFound`, `LoanNotActive`,
`NotAuthorized`, `AlreadyVoted`, `VotingEnded`.

## Alternatives considered

- **Admin-granted extensions.** One key could quietly change members' risk
  and favour particular borrowers. Rejected for the same reason ADR-0001
  rejects admin upgrades.
- **Automatic extension for a penalty rate, no vote.** Members would carry
  extra risk without agreeing to it. `default_grace_period` already provides
  automatic slack.
- **Refinancing with a new loan.** Blocked by `has_active_loan` and the
  cooldown, it would inflate `total_loans`, and it needs treasury liquidity
  that a struggling borrower's loan has already tied up.
- **Pausing defaults while a vote is pending.** Lets a borrower push a
  default back by one voting period whenever they like. The request deadline
  gives the same protection with no new interaction with
  `mark_loan_defaulted`.
- **Unlimited or repeatable extensions.** Invites evergreening. One request
  keeps the outcome bounded; a borrower who needs more time after that
  should default and let the penalty apply.
- **New `LoanPolicy` fields (fee rate, max extensions).** Would break
  decoding of the stored policy. If tuning becomes necessary, a separate
  `ExtensionPolicy` key with defaults is the compatible path.

## Consequences

- **ABI additions:** three entrypoints, the `LoanExtension` type, one
  `ProposalKind` variant, four events and four errors. Nothing existing is
  renamed or renumbered, and the backend indexer needs the new events.
- **Storage:** two new persistent key families, about one record per
  extended loan plus one entry per vote.
- **Cost:** a vote reads every member's delegation entry, like loan votes
  (`members + 11` ledger entries; see the README "Resource costs" section),
  so it shares the existing membership ceiling.
- **Risk:** members can extend a loan that later defaults anyway. The fee
  compensates for time, not for credit risk, and the single request plus
  early deadline limit how long a bad loan can be carried.
- **Follow-up:** implementation with tests for each rule above, including a
  request filed exactly at the deadline, a fee on a partly repaid loan,
  approval followed by repayment (the fee distributed as yield), and
  rejection followed by default.

## References

1. [Loan Rescheduling After a Natural Disaster](https://microfinancegateway.org/paper/1998/01/loan-rescheduling-after-natural-disaster), Microfinance Gateway.
2. [Rapid-onset disasters brief](https://www.gdrc.org/icm/disasters/rapid_onset_brief_1.pdf), GDRC.
