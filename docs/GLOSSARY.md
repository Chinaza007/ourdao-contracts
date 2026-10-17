# OurDAO glossary

| Term | Meaning |
| --- | --- |
| **Admin** | An address authorized to change DAO configuration and pause or unpause the contract. |
| **Consensus threshold** | Basis-point percentage of voting power required to approve a proposal (for example, 5100 = 51%). |
| **DAO token** | The single Stellar token configured at initialization for membership fees, loans, and treasury accounting. |
| **Editing phase** | The initial fixed period during which a borrower may edit a loan proposal. |
| **Voting phase** | The period during which eligible members can vote on a loan or treasury proposal. |
| **Loan proposal** | A member’s request for a loan before it is approved and disbursed. Its identifier is reused by the resulting loan. |
| **Loan policy** | Governance configuration controlling loan limits, interest, duration, grace periods, and default penalties. |
| **Treasury** | Lendable DAO funds after excluding member stakes and other reserved balances. |
| **Member stake** | A member’s deposited membership balance, used in exit-share accounting and (when separately staked) voting boost. |
| **Pending yield** | Interest earned by an active member and claimable through `claim_rewards`. |
| **Staking boost** | The capped additional voting weight earned by locking tokens through `stake`; it is excluded from lendable treasury funds. |
| **Soroban** | Stellar’s smart-contract platform and execution environment used by this repository. |
| **SAC** | Stellar Asset Contract, the Soroban interface for native XLM and Stellar assets. |
| **TTL** | Soroban persistent-storage time-to-live. Reads and writes refresh entries to prevent active state from being archived. |
| **Keeper** | An unprivileged caller that submits an objective maintenance transition, such as marking an overdue loan defaulted. |
| **Commit-reveal** | Voting flow where a hash commitment hides a vote until the voter later reveals the vote and salt. |
| **Content hash** | An on-chain digest identifying off-chain proposal material, such as an IPFS document. |
| **WASM** | The optimized WebAssembly artifact deployed as the Soroban contract. |
