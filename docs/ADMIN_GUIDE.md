# Admin Guide

This guide covers every admin-facing entrypoint in the DAO contract: who may
call it, what it changes, what events it emits, and which errors it can return.
For the meaning of each loan parameter, see
[LOAN_POLICY_GUIDE.md](./LOAN_POLICY_GUIDE.md). For emergency response, see
[INCIDENT_RUNBOOK.md](./INCIDENT_RUNBOOK.md).

---

## Roles

| Role | How it is granted | What it can do |
|---|---|---|
| **Admin** | `initialize` (initial set) or `add_admin` | Every admin entrypoint below, plus `pause`. |
| **Pauser** | `set_pauser` (one address at a time) | `pause` only. Cannot unpause, change policy, or manage admins. |
| **Member** | `register_member` | No admin powers. |

Every admin entrypoint calls `require_auth` on the caller and then checks the
admin list. A non-admin receives `NotAdmin` (error 11). `pause` is the only
entrypoint that also accepts a pauser.

Admin changes take effect immediately. Policy changes do not; they go through a
48-hour timelock (see [Policy updates](#policy-updates-timelocked)).

---

## Entrypoint reference

### `initialize(admins, consensus_threshold, membership_fee, token, policy)`

**Who:** anyone, once. **Effect:** sets the admin list, consensus threshold,
membership fee, DAO token, and loan policy; starts the contract unpaused with
zero members.

**Validation**

- `admins` must be non-empty and contain no duplicates.
- `consensus_threshold` must be in `1..=10_000` basis points.
- `membership_fee` must be greater than zero.
- `policy` must pass the policy checks listed under [`set_loan_policy`](#set_loan_policycaller-policy).
- `token` must answer a read-only `balance` call. A wrong address is rejected
  here rather than failing on the first transfer.

**Event:** `init` with `(admins, consensus_threshold, membership_fee, token)`.

**Errors:** `AlreadyInitialized`, `InvalidThreshold`, `InvalidAmount`,
`NotAuthorized` (no admins), `AlreadyAdmin` (duplicate admin), `InvalidLoanPolicy`,
`InvalidToken`.

---

### `add_admin(caller, admin)`

**Who:** admin. **Effect:** appends `admin` to the admin list.

**Event:** `admin_add` with `admin`.

**Errors:** `NotAdmin`, `AlreadyAdmin`.

---

### `remove_admin(caller, admin)`

**Who:** admin. **Effect:** removes every entry matching `admin`.

The last admin can never be removed. This keeps the DAO from being left with no
one able to pause, unpause, or change policy.

**Event:** `admin_rem` with `admin`.

**Errors:** `NotAdmin` (caller), `CannotRemoveLastAdmin`, `NotAdmin` (target not in the list).

> Removing an admin does not remove any pending policy update they proposed.
> Anyone still in the admin list can execute or cancel it.

---

### `set_pauser(caller, pauser)`

**Who:** admin. **Effect:** sets the single pauser address, replacing any
previous one. The pauser can call `pause` but nothing else admin-only.

**Event:** `pset` with `pauser`.

**Errors:** `NotAdmin`. Also returns `AlreadyPauser` when the address is already
the pauser. **Known issue:** `AlreadyPauser` and `NotPauser` are used in
`admin.rs` but not yet defined in `error.rs`; see the note under
[Error codes referenced here](#error-codes-referenced-here).

Use this to give an on-call responder the ability to stop the contract without
granting them governance power.

---

### `revoke_pauser(caller)`

**Who:** admin. **Effect:** clears the pauser.

**Event:** `prev` with the caller's address.

**Errors:** `NotAdmin`. Also returns `NotPauser` when no pauser is set (same
undefined-variant issue as above).

---

### `set_consensus_threshold(caller, threshold)`

**Who:** admin. **Effect:** sets the member-voting consensus threshold, in
basis points (for example `5_100` = 51%).

**Validation:** `threshold` must be in `1..=10_000`.

**Event:** `threshold` with `threshold`.

**Errors:** `NotAdmin`, `InvalidThreshold`.

---

### Policy updates (timelocked)

Loan policy changes are two steps: propose, then execute after a delay. The
delay is `TIMELOCK_DURATION` = 48 hours (`48 * 60 * 60` seconds). It gives
members time to see a pending change and object before it takes effect.

#### `set_loan_policy(caller, policy)`

**Who:** admin. **Effect:** this is an alias for `propose_policy_update`. It
does **not** change the live policy. It stores a pending update whose
`execution_time` is `now + 48h`, replacing any earlier pending update.

**Validation** (checked at proposal time, and again by `initialize`):

- `membership_contribution` > 0
- `max_loan_duration` > 0
- `min_interest_rate` ≤ `max_interest_rate` ≤ 10_000
- `max_loan_to_treasury_ratio` in `1..=10_000`
- `default_penalty_bps` ≤ 10_000
- `editing_period` and `voting_period` in `1..=30 days`
- `min_membership_duration`, `cooldown_period`, `default_grace_period` ≤ 30 days
- `treasury_threshold` in `1..=10_000`
- `quorum_bps` ≤ 10_000

**Event:** `pol_prop` with `(caller, execution_time)`.

**Errors:** `NotAdmin`, `InvalidLoanPolicy`.

#### `propose_policy_update(caller, policy)`

Same as `set_loan_policy`. Use whichever name your tooling expects.

#### `execute_policy_update(caller)`

**Who:** admin. **Effect:** once `now >= execution_time`, copies the pending
policy into the live policy and clears the pending slot. Anyone in the admin
list can execute it, not only the proposer.

**Event:** `policy` (no payload).

**Errors:** `NotAdmin`, `NoPendingPolicy`, `TimelockNotExpired`.

#### `cancel_policy_update(caller)`

**Who:** admin. **Effect:** discards the pending update. The live policy is
unchanged.

**Event:** `pol_canc` with `caller`.

**Errors:** `NotAdmin`, `NoPendingPolicy`.

#### `get_pending_policy_update()` (view)

Returns the pending update, including `proposed_at` and `execution_time`, or
`None`. Check this before and after a proposal to confirm what members will see.

**Operational note:** the timelock applies per proposal. A second
`set_loan_policy` call replaces the first and restarts the 48-hour clock.

---

### `pause(caller)`

**Who:** admin **or** pauser. **Effect:** sets the global paused flag. While
paused, state-changing entrypoints are blocked. Loan repayments stay available,
so borrowers are not pushed into default by an emergency stop.

The caller must authorize the call directly (`require_auth`).

**Event:** `paused` (no payload).

**Errors:** `NotAdmin` (caller is neither admin nor pauser), `Paused` (already paused).

> `mark_loan_defaulted` is blocked while paused. A loan can pass its grace
> period during a pause and become markable immediately after `unpause`. See
> [INCIDENT_RUNBOOK.md](./INCIDENT_RUNBOOK.md).

---

### `unpause(caller)`

**Who:** admin only. A pauser cannot unpause. **Effect:** clears the paused flag.

**Event:** `unpaused` (no payload).

**Errors:** `NotAdmin`, `NotPaused`.

---

## Quick reference

| Entrypoint | Caller | Immediate? | Event |
|---|---|---|---|
| `initialize` | anyone (once) | Yes | `init` |
| `add_admin` | admin | Yes | `admin_add` |
| `remove_admin` | admin | Yes | `admin_rem` |
| `set_pauser` | admin | Yes | `pset` |
| `revoke_pauser` | admin | Yes | `prev` |
| `set_consensus_threshold` | admin | Yes | `threshold` |
| `set_loan_policy` / `propose_policy_update` | admin | No (48 h) | `pol_prop` |
| `execute_policy_update` | admin | After timelock | `policy` |
| `cancel_policy_update` | admin | Yes | `pol_canc` |
| `pause` | admin or pauser | Yes | `paused` |
| `unpause` | admin | Yes | `unpaused` |

Event names and payloads are part of the public ABI and must not be renamed.
The indexer depends on them.

---

## Operational checklist

- **Keep at least two admins.** `remove_admin` will refuse to remove the last
  one, but two keeps a single lost key from blocking emergency actions.
- **Give responders the pauser role, not admin.** Pausing is fast to undo only
  by an admin, so keep `unpause` with the admin set.
- **Publish policy changes before proposing.** Share the before and after values
  and the worked numbers from [LOAN_POLICY_GUIDE.md](./LOAN_POLICY_GUIDE.md)
  during the 48-hour window.
- **Check the pending update after every proposal** with
  `get_pending_policy_update`, and cancel and re-propose if the values are wrong.
- **Use `get_admins` and `is_admin`** to confirm the admin set after any change.

---

## Error codes referenced here

| Code | Name | Returned by |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` |
| 3 | `InvalidThreshold` | `initialize`, `set_consensus_threshold` |
| 4 | `InvalidAmount` | `initialize` |
| 5 | `InvalidLoanPolicy` | `initialize`, `set_loan_policy` |
| 6 | `Paused` | `pause` |
| 7 | `NotPaused` | `unpause` |
| 10 | `NotAuthorized` | `initialize` (empty admin list) |
| 11 | `NotAdmin` | every admin entrypoint |
| 13 | `AlreadyAdmin` | `initialize`, `add_admin` |
| 15 | `CannotRemoveLastAdmin` | `remove_admin` |
| 75 | `InvalidToken` | `initialize` |
| 76 | `TimelockNotExpired` | `execute_policy_update` |
| 77 | `NoPendingPolicy` | `execute_policy_update`, `cancel_policy_update` |

Codes are from `error.rs`, which is the source of truth. Codes are part of the
public ABI and must never be renumbered.

**Known issue (not fixed in this guide):** `set_pauser` and `revoke_pauser`
return `AlreadyPauser` and `NotPauser`, but neither variant exists in
`error.rs` yet, so the crate does not compile until they are added as new,
appended codes.
