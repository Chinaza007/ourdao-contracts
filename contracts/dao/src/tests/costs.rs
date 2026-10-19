//! Per-entrypoint resource costs behind the README "Resource costs" table (#358).
//!
//! These run the *compiled* release wasm: the SDK only meters VM
//! instantiation, wasm reads and rent when a Wasm contract is invoked, so the
//! native contract the other tests use under-reports every one of them.
//! Ignored by default because the wasm has to be built first:
//!
//! ```text
//! make costs
//! ```
//!
//! `OURDAO_WASM` overrides the wasm path (e.g. to measure an optimized build).

extern crate std;

use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Bytes, Env, String, Vec};
use std::{format, println, string::String as StdString};

use super::common::*;
use crate::storage::ProposalKind;
use crate::types::ProposalStatus;
use crate::OurDaoClient;

const DEFAULT_WASM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../target/wasm32v1-none/release/ourdao_dao.wasm"
);

struct Bench<'a> {
    env: Env,
    client: OurDaoClient<'a>,
    token: token::Client<'a>,
    admin: Address,
    members: Vec<Address>,
}

/// `enforce_limits` keeps the SDK's snapshot of the mainnet per-transaction
/// limits switched on, so any call that would fail on-chain fails here too.
fn wasm_setup(num_members: u32, enforce_limits: bool) -> Bench<'static> {
    let path = std::env::var("OURDAO_WASM").unwrap_or_else(|_| DEFAULT_WASM.into());
    let wasm = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("{path}: {e} (build the wasm first: `make build`)"));

    let env = Env::default();
    env.mock_all_auths();
    if !enforce_limits {
        env.cost_estimate().disable_resource_limits();
    }
    let token_id = env
        .register_stellar_asset_contract_v2(Address::generate(&env))
        .address();
    let token = token::Client::new(&env, &token_id);
    let mint = token::StellarAssetClient::new(&env, &token_id);

    let client = OurDaoClient::new(&env, &env.register(wasm.as_slice(), ()));
    let admin = Address::generate(&env);
    client.initialize(
        &Vec::from_array(&env, [admin.clone()]),
        &5_100u32,
        &FEE,
        &token_id,
        &policy(),
    );

    let mut members = Vec::new(&env);
    for _ in 0..num_members {
        let m = Address::generate(&env);
        mint.mint(&m, &MINT);
        client.register_member(&m);
        members.push_back(m);
    }
    Bench {
        env,
        client,
        token,
        admin,
        members,
    }
}

fn thousands(n: i64) -> StdString {
    let digits = format!("{n}");
    let mut out = StdString::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// One table row for the invocation that just ran. Must be called before any
/// other contract call, since the estimate only covers the last invocation.
fn row(env: &Env, label: &str) -> StdString {
    let r = env.cost_estimate().resources();
    let fee = env.cost_estimate().fee();
    format!(
        "| {label} | {} | {} | {} | {} | {} | {} | {} |",
        thousands(r.instructions),
        thousands(r.mem_bytes / 1024),
        r.memory_read_entries + r.disk_read_entries,
        r.write_entries,
        thousands(r.write_bytes as i64),
        r.contract_events_size_bytes,
        thousands(fee.total),
    )
}

const HEADER: &str = "| Entrypoint | CPU instructions | Memory (KiB) | Entries read | Entries written | Bytes written | Event bytes | Est. fee (stroops) |\n\
                      | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |";

/// Votes `support` from successive members (skipping `skip`) until the
/// proposal leaves `Pending`; returns the row for the vote that decided it.
fn vote_until_decided(
    b: &Bench,
    pid: u32,
    skip: &Address,
    loan: bool,
    label: &str,
) -> (StdString, StdString) {
    let mut first = None;
    for voter in b.members.iter().filter(|m| m != skip) {
        if loan {
            b.client.vote_on_loan_proposal(&voter, &pid, &true);
        } else {
            b.client.vote_on_treasury_proposal(&voter, &pid, &true);
        }
        let this = row(&b.env, label);
        let status = if loan {
            b.client.get_loan_proposal(&pid).unwrap().status
        } else {
            b.client.get_treasury_proposal(&pid).unwrap().status
        };
        if status != ProposalStatus::Pending {
            return (first.unwrap_or_else(|| this.clone()), this);
        }
        first.get_or_insert(this);
    }
    panic!("proposal {pid} was never decided");
}

#[test]
#[ignore = "needs the release wasm: run `make costs`"]
fn entrypoint_resource_costs() {
    let b = wasm_setup(10, true);
    let m = |i: u32| b.members.get(i).unwrap();
    let dest = Address::generate(&b.env);
    let mut table = StdString::from(HEADER);
    let mut push = |r: StdString| {
        table.push('\n');
        table.push_str(&r);
    };

    // Membership & governance.
    let newcomer = Address::generate(&b.env);
    token::StellarAssetClient::new(&b.env, &b.token.address).mint(&newcomer, &MINT);
    b.client.register_member(&newcomer);
    push(row(&b.env, "`register_member`"));
    b.client.stake(&m(1), &500);
    push(row(&b.env, "`stake`"));
    // `delegate_vote` is left out: it calls `require_auth` twice on the
    // delegator and also demands the delegatee's auth, so it traps with
    // `Error(Auth, ExistingValue)` before doing any work.
    b.client
        .register_name(&m(2), &String::from_str(&b.env, "alice"));
    push(row(&b.env, "`register_name`"));
    b.client.add_admin(&b.admin, &Address::generate(&b.env));
    push(row(&b.env, "`add_admin`"));
    b.client.propose_policy_update(&b.admin, &policy());
    push(row(&b.env, "`propose_policy_update`"));

    // Loan lifecycle: request -> edit -> vote -> disburse -> repay.
    let pid = b.client.request_loan(&m(0), &1_000, &None);
    push(row(&b.env, "`request_loan`"));
    b.client.attach_document(
        &m(0),
        &ProposalKind::Loan,
        &pid,
        &Bytes::from_array(&b.env, &[0xab; 32]),
    );
    push(row(&b.env, "`attach_document`"));
    b.client.edit_loan_proposal(&m(0), &pid, &900);
    push(row(&b.env, "`edit_loan_proposal`"));
    advance(&b.env, EDITING + 1);
    let (first, deciding) = vote_until_decided(&b, pid, &m(0), true, "`vote_on_loan_proposal`");
    push(first);
    push(deciding.replacen(
        "`vote_on_loan_proposal`",
        "`vote_on_loan_proposal` (deciding vote, disburses)",
        1,
    ));
    b.client.repay_loan_partial(&m(0), &pid, &100);
    push(row(&b.env, "`repay_loan_partial`"));
    b.client.repay_loan(&m(0), &pid);
    push(row(&b.env, "`repay_loan`"));
    b.client.claim_rewards(&m(1));
    push(row(&b.env, "`claim_rewards`"));

    // A second loan that is left to default.
    let late = b.client.request_loan(&m(4), &500, &None);
    advance(&b.env, EDITING + 1);
    vote_until_decided(&b, late, &m(4), true, "");
    advance(&b.env, LOAN_DURATION + 1);
    b.client.mark_loan_defaulted(&late);
    push(row(&b.env, "`mark_loan_defaulted`"));

    // Treasury withdrawal: propose -> vote -> execute on the deciding vote.
    let tid = b.client.propose_treasury_withdrawal(
        &m(3),
        &500,
        &dest,
        &String::from_str(&b.env, "grant"),
        &false,
    );
    push(row(&b.env, "`propose_treasury_withdrawal`"));
    let (first, deciding) =
        vote_until_decided(&b, tid, &m(3), false, "`vote_on_treasury_proposal`");
    push(first);
    push(deciding.replacen(
        "`vote_on_treasury_proposal`",
        "`vote_on_treasury_proposal` (deciding vote, pays out)",
        1,
    ));

    b.client.unstake(&m(1), &500);
    push(row(&b.env, "`unstake`"));
    b.client.exit_dao(&m(5));
    push(row(&b.env, "`exit_dao`"));

    // Read-only views (simulation-only when called off-chain).
    b.client.get_member(&m(1));
    push(row(&b.env, "`get_member`"));
    b.client.get_member_loan_stats(&m(0));
    push(row(&b.env, "`get_member_loan_stats`"));
    b.client.get_voting_weight(&m(1));
    push(row(&b.env, "`get_voting_weight`"));
    b.client.calculate_exit_share(&m(1));
    push(row(&b.env, "`calculate_exit_share`"));
    b.client.is_eligible_for_loan(&m(6));
    push(row(&b.env, "`is_eligible_for_loan`"));
    b.client.get_treasury_balance();
    push(row(&b.env, "`get_treasury_balance`"));

    println!("\n{table}\n");
}

/// How the member-count-sensitive entrypoints grow, as `CPU instructions /
/// ledger entries touched` at exactly `n` members (`register_member` is the
/// n-th join). Limits are off so the table can show where calls
/// cross the SDK's mainnet snapshot (100 entries per transaction); those cells
/// are marked "over".
#[test]
#[ignore = "needs the release wasm: run `make costs`"]
fn entrypoint_cost_scaling() {
    let mut table = StdString::from(
        "| Members | `get_total_members` | `register_member` | `vote_on_loan_proposal` | `vote_on_treasury_proposal` | `exit_dao` |\n\
         | ---: | ---: | ---: | ---: | ---: | ---: |",
    );
    for n in [10u32, 25, 50, 85, 86, 89, 90, 100] {
        let b = wasm_setup(n, false);
        let m = |i: u32| b.members.get(i).unwrap();
        let cell = |b: &Bench| {
            let r = b.env.cost_estimate().resources();
            let entries = r.memory_read_entries + r.disk_read_entries + r.write_entries;
            let over = if entries > 100 { " (over)" } else { "" };
            format!("{} / {entries}{over}", thousands(r.instructions))
        };

        // The last setup step was the n-th `register_member`.
        let join = cell(&b);
        b.client.get_total_members();
        let view = cell(&b);
        let pid = b.client.request_loan(&m(0), &100, &None);
        advance(&b.env, EDITING + 1);
        b.client.vote_on_loan_proposal(&m(1), &pid, &true);
        let loan_vote = cell(&b);
        let tid = b.client.propose_treasury_withdrawal(
            &m(2),
            &100,
            &m(3),
            &String::from_str(&b.env, "grant"),
            &false,
        );
        b.client.vote_on_treasury_proposal(&m(4), &tid, &true);
        let treasury_vote = cell(&b);
        b.client.exit_dao(&m(5));
        let exit = cell(&b);

        table.push_str(&format!(
            "\n| {n} | {view} | {join} | {loan_vote} | {treasury_vote} | {exit} |"
        ));
    }
    println!("\nCPU instructions / ledger entries touched, by DAO size:\n\n{table}\n");
}
