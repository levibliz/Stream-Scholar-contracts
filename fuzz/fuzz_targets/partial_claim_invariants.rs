// Exhaustively fuzz the `withdraw_scholarship` partial-claim path.
//
// Drives `claim_math::execute_partial_claim` with an arbitrary scholarship
// state plus a requested withdrawal amount. The target enforces three classes
// of invariant on every iteration:
//
//   1. Accept/reject decision is consistent with the contract's accept rules
//      (the same conditions that produce a `panic!` inline must produce a
//      `ClaimError` here).
//   2. On a successful claim, the post-state preserves the value invariant
//      `new_balance + gross_amount == old_balance` and `tax + net == gross`.
//   3. The locked-amount window is never crossed unless `final_release_claimed`
//      is set.
//
// Edge cases the corpus is biased toward (anchor selectors below):
//   * balances/grants near 0, near i128::MAX, exactly equal to the locked
//     boundary, and one unit on either side of the boundary;
//   * tax rate at 0, 1bps, 9_999bps, and exactly 10_000bps (full tax);
//   * negative `unlocked_balance` (corrupted state) — should still not
//     produce a panic from the pure-math layer.

#![no_main]

use arbitrary::Arbitrary;
use claim_math::{
    apply_bps_tax, available_to_withdraw, execute_partial_claim,
    final_release_locked_amount, ClaimError, BPS_DENOMINATOR,
};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    // Anchored fields — biased toward boundary values.
    balance_anchor: u8,
    balance_offset: i64,
    unlocked_anchor: u8,
    unlocked_offset: i64,
    grant_anchor: u8,
    grant_offset: i64,
    requested_anchor: u8,
    requested_offset: i64,
    tax_anchor: u8,
    tax_offset: u32,
    final_release_claimed: bool,
}

/// Anchors the fuzzer toward boundary values for i128 amounts: 0, small
/// numbers, locked-edge-ish, near i64::MAX (so multiplications still fit),
/// and a few negatives so we cover corrupted-state inputs.
fn anchor_i128(anchor: u8, offset: i64) -> i128 {
    match anchor % 8 {
        0 => 0,
        1 => offset.unsigned_abs() as i128,
        2 => (offset as i128).saturating_mul(1_000_000),
        3 => 100i128 + offset as i128,                 // near locked-edge for grant=1000
        4 => 1_000i128 + offset as i128,               // near locked-edge for grant=10_000
        5 => i64::MAX as i128 + offset as i128,        // headroom for *10
        6 => -(offset.unsigned_abs() as i128),         // negative
        _ => offset as i128,
    }
}

/// Anchors basis-point tax rates to the boundary values that matter.
fn anchor_tax_bps(anchor: u8, offset: u32) -> u32 {
    match anchor % 6 {
        0 => 0,
        1 => 1,
        2 => 100,
        3 => 9_999,
        4 => 10_000,
        _ => offset % 20_000, // covers >100% which the math should reject
    }
}

fuzz_target!(|input: FuzzInput| {
    let balance = anchor_i128(input.balance_anchor, input.balance_offset);
    let unlocked = anchor_i128(input.unlocked_anchor, input.unlocked_offset);
    let grant = anchor_i128(input.grant_anchor, input.grant_offset);
    let requested = anchor_i128(input.requested_anchor, input.requested_offset);
    let tax_bps = anchor_tax_bps(input.tax_anchor, input.tax_offset);
    let final_claimed = input.final_release_claimed;

    // Invariant: `final_release_locked_amount` must never panic. It either
    // returns a non-negative number (for non-negative grants) or `None` on
    // overflow.
    if let Some(locked) = final_release_locked_amount(grant) {
        if grant >= 0 {
            assert!(locked >= 0);
            // Locked is exactly 10% (floored).
            if let Some(prod) = grant.checked_mul(10) {
                assert_eq!(locked, prod / 100);
            }
        }
    }

    // Invariant: `apply_bps_tax` either returns `(net, tax)` summing to
    // `amount`, or rejects the input on overflow / over-100% rates.
    if let Some((net, tax)) = apply_bps_tax(requested, tax_bps) {
        if tax_bps <= BPS_DENOMINATOR as u32 {
            assert_eq!(
                net.checked_add(tax),
                Some(requested),
                "value lost in tax: req={} bps={} net={} tax={}",
                requested,
                tax_bps,
                net,
                tax
            );
            // Tax is non-negative for non-negative requests.
            if requested >= 0 {
                assert!(tax >= 0 && net >= 0);
                assert!(tax <= requested);
            }
        }
    }

    // Invariant: `available_to_withdraw` returns either a valid available
    // amount or `None` on overflow. It never returns a value that, when
    // subtracted from `balance`, would cross the locked window.
    if let Some(available) =
        available_to_withdraw(unlocked, balance, grant, final_claimed)
    {
        assert!(available >= 0);
        if !final_claimed && grant > 0 {
            if let Some(locked) = final_release_locked_amount(grant) {
                // Withdrawing all of `available` must leave at least `locked`
                // behind unless balance was below locked already (in which
                // case available is 0).
                if balance > locked {
                    assert!(balance.saturating_sub(available) >= locked);
                } else {
                    assert_eq!(available, 0);
                }
            }
        }
    }

    // Invariant: `execute_partial_claim` either succeeds with consistent
    // post-state, or returns a structured `ClaimError` matching the
    // contract's reject conditions.
    match execute_partial_claim(unlocked, balance, grant, final_claimed, requested, tax_bps) {
        Ok(claim) => {
            // Value-conservation: net + tax = gross.
            assert_eq!(
                claim.net_amount.checked_add(claim.tax_amount),
                Some(claim.gross_amount),
                "value lost in execute_partial_claim"
            );
            // Post-state is the pre-state minus the gross.
            assert_eq!(claim.new_balance, balance - requested);
            assert_eq!(claim.new_unlocked_balance, unlocked - requested);
            // Locked-window invariant.
            if !final_claimed && grant > 0 {
                let locked =
                    final_release_locked_amount(grant).expect("locked computed already");
                assert!(claim.new_balance >= locked);
            }
            // Net is non-negative for non-negative request.
            assert!(claim.net_amount >= 0);
            assert!(claim.tax_amount >= 0);
            // Gross does not exceed available.
            let avail = available_to_withdraw(unlocked, balance, grant, final_claimed)
                .expect("available computed");
            assert!(claim.gross_amount <= avail);
        }
        Err(ClaimError::InvalidAmount) => {
            assert!(requested <= 0);
        }
        Err(ClaimError::FinalReleaseLocked) => {
            let locked = final_release_locked_amount(grant).unwrap_or(0);
            assert!(balance <= locked);
            assert!(!final_claimed);
        }
        Err(ClaimError::ExceedsAvailable) => {
            let avail = available_to_withdraw(unlocked, balance, grant, final_claimed)
                .unwrap_or(0);
            assert!(requested > avail);
        }
        Err(ClaimError::InsufficientBalance) => {
            assert!(balance < requested);
        }
        Err(ClaimError::Overflow) => {
            // Acceptable: an intermediate calculation overflowed. The
            // contract's safe_math wrappers will translate this into a
            // `MathErr` panic on-chain.
        }
    }
});
