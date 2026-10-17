# OurDAO member guide

This guide describes the normal member lifecycle. The examples use the
OurDAO contract deployed for the selected network; see
[`DEPLOYMENTS.md`](../DEPLOYMENTS.md) for contract IDs and the configured DAO
token.

## 1. Join the DAO

1. Connect a Stellar wallet and make sure it is funded with the DAO token and
   enough XLM for transaction fees.
2. Read the membership fee with `get_membership_fee` (or the deployment’s
   equivalent view) and confirm the token is the configured DAO token.
3. Submit `register_member(member)`. The contract transfers the membership fee
   into the treasury and emits `joined`.
4. Confirm membership with `is_member(member)` and inspect the recorded balance
   with `get_member(member)`.

Joining is a state-changing transaction, so wait for confirmation before
starting another member operation. Never send the fee directly to the token
contract; use the contract entrypoint so the member record is created.

## 2. Borrow

1. Check `is_eligible_for_loan(member)`, `get_loan_policy()`, and
   `get_treasury_balance()` before requesting.
2. Call `request_loan(borrower, amount)`. Save the returned `proposal_id`.
3. During the editing phase, the borrower may call
   `edit_loan_proposal(borrower, proposal_id, new_amount)`.
4. During voting, eligible members call
   `vote_on_loan_proposal(voter, proposal_id, support)` once each.
5. When the consensus threshold is reached, approval and disbursement happen
   automatically. Inspect `get_loan(proposal_id)` and the `loan_appr` event.
6. Repay with `repay_loan(borrower, proposal_id)` or make a partial payment
   with `repay_loan_partial(borrower, proposal_id, amount)`. Interest is paid
   before principal, and the loan is complete only when its balance reaches
   zero.

If the proposal expires, anyone may call `expire_loan_proposal`. If a loan is
overdue after its grace period, anyone may call `mark_loan_defaulted`; this is
the keeper path and does not require admin authorization.

## 3. Vote

Read the proposal and the current policy before signing. Your voting weight is
based on membership stake and any eligible staking boost. A public proposal is
voted on with the appropriate `vote_on_loan_proposal` or
`vote_on_treasury_proposal` call. Treasury proposals may be private: use
`commit_treasury_vote` first, then `reveal_treasury_vote` during the reveal
window with the same support value and salt.

Each voter can vote only once per proposal. Review the transaction simulation,
destination, amount, and proposal ID before approving it in the wallet.

## 4. Exit

1. Confirm the member has no active loan and decide whether to claim pending
   yield separately.
2. Optionally call `claim_rewards(member)` to withdraw accrued interest.
3. Call `exit_dao(member)`. The contract calculates the member’s pro-rata
   treasury share, returns the membership balance and stake, marks the member
   inactive, and emits `exited`.
4. Confirm the member is no longer active with `is_member(member)`.

Exiting is irreversible for that membership record, but a former member may
join again by paying the membership fee. Keep the confirmed transaction hash
for support and never sign an exit while a loan is active.

## Safety checklist

- Verify the network passphrase and contract ID from `DEPLOYMENTS.md`.
- Inspect every transaction simulation and wallet authorization entry.
- Keep the wallet seed phrase private; maintainers will never request it.
- If the contract is paused, wait for an administrator’s announcement instead
  of repeatedly submitting state-changing transactions.
