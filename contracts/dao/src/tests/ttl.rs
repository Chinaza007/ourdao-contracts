use soroban_sdk::testutils::storage::Persistent as _;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Bytes, BytesN, String};

use super::common::*;
use crate::storage::{DataKey, ProposalKind};

const DAY_IN_LEDGERS: u32 = 17_280;
const PERSISTENT_BUMP: u32 = 90 * DAY_IN_LEDGERS;

#[test]
fn ttl_extended_for_member_on_write_and_read() {
    let s = setup(1);
    let member = s.members.get(0).unwrap();

    let key = DataKey::Member(member.clone());
    let ttl_after_write = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_write, PERSISTENT_BUMP);

    // Advance ledger sequence past the threshold so TTL drops below PERSISTENT_THRESHOLD.
    let advance_ledgers = DAY_IN_LEDGERS + 10;
    s.env.ledger().with_mut(|li| {
        li.sequence_number += advance_ledgers;
    });
    let ttl_after_advance = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_advance, PERSISTENT_BUMP - advance_ledgers);

    // Reading the member must extend the TTL back to PERSISTENT_BUMP.
    let m = s.client.get_member(&member);
    assert!(m.is_some());
    let ttl_after_read = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_read, PERSISTENT_BUMP);
}

#[test]
fn ttl_extended_for_loan_on_write_and_read() {
    let s = setup(3);
    let borrower = s.members.get(0).unwrap();
    let v1 = s.members.get(1).unwrap();
    let v2 = s.members.get(2).unwrap();

    let pid = s.client.request_loan(&borrower, &1_000, &None);
    advance(&s.env, EDITING + 1);
    s.client.vote_on_loan_proposal(&v1, &pid, &true);
    s.client.vote_on_loan_proposal(&v2, &pid, &true);

    let key = DataKey::Loan(pid);
    let ttl_after_write = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_write, PERSISTENT_BUMP);

    // Advance ledger sequence past the threshold so TTL drops below PERSISTENT_THRESHOLD.
    let advance_ledgers = DAY_IN_LEDGERS + 10;
    s.env.ledger().with_mut(|li| {
        li.sequence_number += advance_ledgers;
    });
    let ttl_after_advance = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_advance, PERSISTENT_BUMP - advance_ledgers);

    // Reading the loan must extend the TTL back to PERSISTENT_BUMP.
    let loan = s.client.get_loan(&pid);
    assert!(loan.is_some());
    let ttl_after_read = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl_after_read, PERSISTENT_BUMP);
}

#[test]
fn ttl_extended_for_stake_on_write() {
    let s = setup(1);
    let member = s.members.get(0).unwrap();
    s.client.stake(&member, &500);

    let key = DataKey::Stake(member.clone());
    let ttl = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl, PERSISTENT_BUMP);
}

#[test]
fn ttl_extended_for_commit_on_write() {
    let s = setup(3);
    let proposer = s.members.get(0).unwrap();
    let voter = s.members.get(1).unwrap();
    let dest = Address::generate(&s.env);
    let reason = String::from_str(&s.env, "private withdrawal");

    let pid = s
        .client
        .propose_treasury_withdrawal(&proposer, &500, &dest, &reason, &true);

    let commitment = BytesN::from_array(&s.env, &[42u8; 32]);
    s.client.commit_treasury_vote(&voter, &pid, &commitment);

    let key = DataKey::Commit(pid, voter.clone());
    let ttl = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl, PERSISTENT_BUMP);
}

#[test]
fn ttl_extended_for_doc_on_write() {
    let s = setup(1);
    let borrower = s.members.get(0).unwrap();
    let pid = s.client.request_loan(&borrower, &500, &None);

    let doc_hash = Bytes::from_slice(
        &s.env,
        b"ipfs://QmXoypizjW3WknFiJnKLwHCnL72vedxjQkDDP1mXWo6uco",
    );
    s.client
        .attach_document(&borrower, &ProposalKind::Loan, &pid, &doc_hash);

    let key = DataKey::Doc(ProposalKind::Loan, pid);
    let ttl = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&key)
    });
    assert_eq!(ttl, PERSISTENT_BUMP);
}

#[test]
fn ttl_extended_for_name_on_write() {
    let s = setup(1);
    let member = s.members.get(0).unwrap();
    let name = String::from_str(&s.env, "alice");

    s.client.register_name(&member, &name);

    let name_key = DataKey::Name(name.clone());
    let ttl_name = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&name_key)
    });
    assert_eq!(ttl_name, PERSISTENT_BUMP);

    let owner_key = DataKey::NameOf(member.clone());
    let ttl_owner = s.env.as_contract(&s.client.address, || {
        s.env.storage().persistent().get_ttl(&owner_key)
    });
    assert_eq!(ttl_owner, PERSISTENT_BUMP);
}
