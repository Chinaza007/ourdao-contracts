/// Migration seed entrypoints (#332 / #333).
///
/// These functions are only callable during the pre-activation window — the
/// period between `initialize()` and the first call to `lock_seeding()`. They
/// let an admin populate a freshly-deployed contract with state migrated from a
/// decommissioned deployment, as described in `docs/MIGRATION.md`.
///
/// All seed functions are:
///   - Admin-only (`require_admin`).
///   - Gated behind the `MigrationSeedingLocked` flag; rejected once
///     `lock_seeding()` has been called.
///   - Idempotent per key: re-seeding the same address/id overwrites, so a
///     bad batch can be corrected by re-sending it before `lock_seeding`.
use soroban_sdk::{Env, Vec};

use crate::error::Error;
use crate::storage::{self};
use crate::types::{
    Loan, Member, MemberStatus, SeedDocEntry, SeedNameEntry, SeedStakeEntry, SeedYieldEntry,
};
use crate::util;

fn require_seeding_open(env: &Env) -> Result<(), Error> {
    if storage::is_seeding_locked(env) {
        Err(Error::SeedingLocked)
    } else {
        Ok(())
    }
}

/// Seeds member records. Maintains the address list and aggregate counters
/// (`TotalMembers`, `ActiveMembers`, `ActiveContributions`) correctly for
/// idempotent batches: if the member is already in the list its previous
/// contribution is subtracted before the new record is written.
#[allow(deprecated)]
pub fn seed_members(
    env: &Env,
    caller: soroban_sdk::Address,
    members: Vec<Member>,
) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    let mut member_list = storage::get_members(env);
    let mut total = storage::get_total_members(env);
    let mut active = storage::get_active_members(env);
    let mut active_contributions = storage::get_active_contributions(env);

    for m in members.iter() {
        let already = storage::get_member(env, &m.address);

        // Undo counters for the existing record before overwriting.
        if let Some(ref prev) = already {
            if prev.status == MemberStatus::ActiveMember {
                active = active.saturating_sub(1);
                active_contributions =
                    active_contributions.saturating_sub(prev.contribution.max(0));
            }
            total = total.saturating_sub(1);
        } else {
            // New address — add to the list.
            member_list.push_back(m.address.clone());
        }

        // Apply counters for the incoming record.
        total += 1;
        if m.status == MemberStatus::ActiveMember {
            active += 1;
            active_contributions += m.contribution.max(0);
        }

        storage::set_member(env, &m);
    }

    storage::set_members(env, &member_list);
    storage::set_total_members(env, total);
    storage::set_active_members(env, active);
    storage::set_active_contributions(env, active_contributions);

    Ok(())
}

pub fn seed_stakes(
    env: &Env,
    caller: soroban_sdk::Address,
    stakes: Vec<SeedStakeEntry>,
) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    let mut total_staked = storage::get_total_staked(env);
    for entry in stakes.iter() {
        let prev = storage::get_stake(env, &entry.address);
        total_staked = total_staked.saturating_sub(prev) + entry.amount.max(0);
        storage::set_stake(env, &entry.address, entry.amount);
    }
    storage::set_total_staked(env, total_staked);

    Ok(())
}

/// Seeds pending yield. Per `docs/MIGRATION.md`: leaves the accumulator at 0
/// and stores `-amount` as the member's yield snapshot, so the formula
/// `(accumulator - snapshot).max(0)` reconstructs the correct pending yield.
pub fn seed_pending_yield(
    env: &Env,
    caller: soroban_sdk::Address,
    yields: Vec<SeedYieldEntry>,
) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    for entry in yields.iter() {
        storage::set_yield_snapshot(env, &entry.address, -entry.amount.max(0));
    }

    Ok(())
}

/// Seeds loan records. Advances `NextProposalId` past every seeded loan id so
/// fresh proposals never collide with seeded ones.
pub fn seed_loans(env: &Env, caller: soroban_sdk::Address, loans: Vec<Loan>) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    for loan in loans.iter() {
        storage::set_loan(env, &loan);
        // Ensure the next-id counter won't reuse this id.
        storage::set_next_proposal_id_at_least(env, loan.id + 1);
    }

    Ok(())
}

pub fn seed_names(
    env: &Env,
    caller: soroban_sdk::Address,
    names: Vec<SeedNameEntry>,
) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    for entry in names.iter() {
        storage::set_name(env, &entry.name, &entry.owner);
    }

    Ok(())
}

pub fn seed_docs(
    env: &Env,
    caller: soroban_sdk::Address,
    docs: Vec<SeedDocEntry>,
) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    require_seeding_open(env)?;
    storage::extend_instance(env);

    for entry in docs.iter() {
        storage::set_doc(env, entry.kind, entry.proposal_id, &entry.content_hash);
        match entry.kind {
            crate::storage::ProposalKind::Loan => {
                storage::set_next_proposal_id_at_least(env, entry.proposal_id + 1);
            }
            crate::storage::ProposalKind::Treasury => {
                storage::set_next_treasury_id_at_least(env, entry.proposal_id + 1);
            }
        }
    }

    Ok(())
}

/// Irreversibly closes the seeding window. All seed_* functions become no-ops
/// after this. Call only once independent verification (procedure step 8 in
/// `docs/MIGRATION.md`) is complete.
pub fn lock_seeding(env: &Env, caller: soroban_sdk::Address) -> Result<(), Error> {
    util::require_admin(env, &caller)?;
    if storage::is_seeding_locked(env) {
        return Err(Error::SeedingAlreadyLocked);
    }
    storage::extend_instance(env);
    storage::lock_seeding(env);
    Ok(())
}
