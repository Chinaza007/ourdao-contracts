//! Budget scaling benchmark (#138, extended for multi-token entrypoints in #354).
//!
//! Run with `scripts/bench-cost.sh`. For each DAO size it measures the CPU
//! instruction cost of one call to every benchmarked entrypoint and prints a
//! `BENCH <entrypoint> <size> <cpu>` line. If an entrypoint's cost at the
//! largest size exceeds `MAX_GROWTH` times its cost at the smallest size, a
//! `REGRESSION` line is printed, which `bench-cost.sh` turns into a failure.

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};

use crate::tests::common::*;

const SIZES: [u32; 3] = [10, 100, 1000];
/// An O(1) entrypoint should stay roughly flat as the DAO grows; allow slack
/// for the larger storage footprint and a handful of extra ledger entries.
const MAX_GROWTH: u64 = 3;

fn cpu(env: &Env) -> u64 {
    env.cost_estimate().budget().cpu_instruction_cost()
}

fn reset(env: &Env) {
    env.cost_estimate().budget().reset_default();
}

/// Votes `pid` through to approval so the loan is disbursed. Voting stops as
/// soon as the proposal is approved, so only the needed votes are cast.
fn approve_loan(s: &Setup, pid: u32, skip: u32) {
    advance(&s.env, EDITING + 1);
    for i in 0..s.members.len() {
        if i == skip {
            continue;
        }
        let voter = s.members.get(i).unwrap();
        if s.client.try_vote_on_loan_proposal(&voter, &pid, &true).is_err() {
            break;
        }
        if s.client.get_loan(&pid).is_some() {
            break;
        }
    }
}

/// Returns `(entrypoint, cpu)` pairs for one DAO size.
fn measure(size: u32) -> [(&'static str, u64); 4] {
    let s = setup(size);
    let asset = whitelist_second_asset(&s, 1_000_000_000);
    let borrower = s.members.get(0).unwrap();

    // propose_withdrawal_in_asset
    let dest = Address::generate(&s.env);
    let reason = String::from_str(&s.env, "bench");
    reset(&s.env);
    s.client.propose_withdrawal_in_asset(
        &borrower,
        &1_000,
        &asset.address,
        &dest,
        &reason,
        &false,
    );
    let propose = cpu(&s.env);

    // request_loan_in_asset
    reset(&s.env);
    let pid = s
        .client
        .request_loan_in_asset(&borrower, &1_000, &asset.address, &None);
    let request = cpu(&s.env);

    // repay_loan_in_asset
    approve_loan(&s, pid, 0);
    let loan = s.client.get_loan(&pid).expect("loan was not approved");
    mint_asset(&s, &asset.address, &borrower, loan.total_repayment);
    reset(&s.env);
    s.client
        .repay_loan_in_asset(&borrower, &pid, &asset.address);
    let repay = cpu(&s.env);

    // exit_dao (the original #138 subject, kept for comparison)
    let leaver = s.members.get(1).unwrap();
    reset(&s.env);
    s.client.exit_dao(&leaver);
    let exit = cpu(&s.env);

    [
        ("request_loan_in_asset", request),
        ("repay_loan_in_asset", repay),
        ("propose_withdrawal_in_asset", propose),
        ("exit_dao", exit),
    ]
}

#[test]
fn test_budget_scaling() {
    let results: std::vec::Vec<_> = SIZES.iter().map(|&n| (n, measure(n))).collect();

    for (n, rows) in &results {
        for (name, c) in rows {
            std::println!("BENCH {name} {n} {c}");
        }
    }

    let (small, first) = &results[0];
    let (large, last) = &results[results.len() - 1];
    for i in 0..first.len() {
        let (name, lo) = first[i];
        let (_, hi) = last[i];
        if hi > lo.max(1) * MAX_GROWTH {
            std::println!(
                "REGRESSION {name}: cpu grew from {lo} (n={small}) to {hi} (n={large})"
            );
        }
    }
}
