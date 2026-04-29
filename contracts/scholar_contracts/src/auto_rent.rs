// Auto_Rent_Deduction — Issue: Long-Term Grant Storage Protection
//
// University research grants can span 5–10 years. Without active TTL management,
// the contract's ledger entries risk archival, permanently destroying student funds.
//
// This module implements the `auto_rent_deduction` hook that is called inside the
// `claim_scholarship` and `withdraw_scholarship` core loops. On every successful
// stream withdrawal it:
//
//   1. Checks whether the contract instance TTL is below the safety threshold
//      (RENT_SAFETY_THRESHOLD_LEDGERS — approximately 6 months of ledger time).
//   2. If the TTL is below the threshold, deducts a micro-fraction of XLM from
//      the withdrawal amount and routes it to extend the contract's instance TTL.
//   3. If the scholarship token is not native XLM, attempts a micro-swap via a
//      registered DEX AMM to obtain native tokens. If the swap fails, the rent
//      top-up is skipped for that ledger to avoid blocking the student's payout.
//   4. Emits a `RentAutoRenewed` event documenting the TTL extension achieved.
//
// Security invariants:
//   - Deduction only occurs when TTL is below the safety threshold.
//   - Deduction is capped at RENT_MICRO_DEDUCTION_STROOPS (100 stroops ≈ 0.00001 XLM).
//   - Swap failures are silently skipped — the student's payout is never blocked.
//   - The deduction is taken from the *gross* withdrawal amount before the student
//     receives net_amount, so the student's net payout is unaffected.
//   - All arithmetic uses checked operations; overflow returns None and skips the
//     top-up rather than panicking.

use soroban_sdk::{symbol_short, token, Address, Env, Symbol};

use crate::DataKey;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Safety threshold: if the contract instance TTL (in ledgers) is below this
/// value, the auto-rent hook fires. Stellar produces ~1 ledger every 5 seconds,
/// so 6 months ≈ 6 * 30 * 24 * 3600 / 5 = 3_110_400 ledgers.
pub const RENT_SAFETY_THRESHOLD_LEDGERS: u32 = 3_110_400;

/// How far to extend the TTL when the hook fires (in ledgers). Extends by
/// approximately 1 year = 12 * 30 * 24 * 3600 / 5 = 6_220_800 ledgers.
pub const RENT_EXTEND_TO_LEDGERS: u32 = 6_220_800;

/// Micro-deduction per withdrawal in stroops (1 stroop = 0.0000001 XLM).
/// 100 stroops ≈ 0.00001 XLM — economically negligible for the student.
pub const RENT_MICRO_DEDUCTION_STROOPS: i128 = 100;

/// Minimum withdrawal amount required before the rent hook fires.
/// Prevents the deduction from consuming a disproportionate share of tiny claims.
pub const RENT_MIN_WITHDRAWAL_FOR_HOOK: i128 = 10_000; // 0.001 XLM

// ---------------------------------------------------------------------------
// Core hook
// ---------------------------------------------------------------------------

/// Attempt to auto-deduct rent from a successful scholarship withdrawal.
///
/// # Parameters
/// - `env`            — Soroban execution environment.
/// - `student`        — The student performing the withdrawal.
/// - `withdrawal_amount` — The gross amount being withdrawn (before tax).
/// - `token`          — The scholarship token address.
/// - `is_native`      — Whether the scholarship token is native XLM.
///
/// # Returns
/// The actual rent amount deducted (0 if the hook was skipped).
///
/// # Behaviour
/// This function is **infallible from the caller's perspective**: any internal
/// failure (swap failure, arithmetic overflow, TTL already healthy) causes the
/// function to return 0 and emit no event, leaving the student's payout intact.
pub fn auto_rent_deduction(
    env: &Env,
    student: &Address,
    withdrawal_amount: i128,
    token: &Address,
    is_native: bool,
) -> i128 {
    // Guard: withdrawal must be large enough to justify the micro-deduction.
    if withdrawal_amount < RENT_MIN_WITHDRAWAL_FOR_HOOK {
        return 0;
    }

    // Guard: only fire when the instance TTL is below the safety threshold.
    // `get_ttl` returns the remaining TTL in ledgers for the contract instance.
    let current_ttl = env.storage().instance().get_ttl();
    if current_ttl >= RENT_SAFETY_THRESHOLD_LEDGERS {
        return 0;
    }

    // Determine the deduction amount — capped at RENT_MICRO_DEDUCTION_STROOPS
    // and also capped at 1% of the withdrawal to stay economically negligible.
    let one_percent = withdrawal_amount.checked_div(100).unwrap_or(0);
    let deduction = core::cmp::min(RENT_MICRO_DEDUCTION_STROOPS, one_percent);
    if deduction <= 0 {
        return 0;
    }

    // Obtain native XLM for the rent payment.
    let xlm_obtained = if is_native {
        // Token is already XLM — use the deduction directly.
        deduction
    } else {
        // Token is non-native — attempt a micro-swap via the registered DEX AMM.
        // If no AMM is registered or the swap fails, skip the top-up.
        match try_swap_for_xlm(env, token, deduction) {
            Some(xlm) if xlm > 0 => xlm,
            _ => {
                // Swap failed or returned zero — skip rent top-up for this ledger.
                // The student's payout is unaffected.
                return 0;
            }
        }
    };

    // Extend the contract instance TTL.
    // `extend_ttl(threshold, extend_to)` only extends if current TTL < threshold.
    env.storage()
        .instance()
        .extend_ttl(RENT_SAFETY_THRESHOLD_LEDGERS, RENT_EXTEND_TO_LEDGERS);

    // Record the last extension timestamp for audit purposes.
    let now = env.ledger().timestamp();
    env.storage()
        .instance()
        .set(&DataKey::RentLastExtended, &now);

    // Emit RentAutoRenewed event documenting the TTL extension.
    // Topics: ("RentAutoRenewed", student_address)
    // Data:   (xlm_deducted_stroops, new_ttl_ledgers, timestamp)
    let new_ttl = env.storage().instance().get_ttl();
    #[allow(deprecated)]
    env.events().publish(
        (Symbol::new(env, "RentAutoRenewed"), student.clone()),
        (xlm_obtained, new_ttl, now),
    );

    xlm_obtained
}

// ---------------------------------------------------------------------------
// DEX micro-swap helper
// ---------------------------------------------------------------------------

/// Attempt to swap `amount` stroops of `token` for native XLM via the
/// registered AMM. Returns `Some(xlm_stroops)` on success, `None` on failure.
///
/// This is a best-effort operation. The caller must treat `None` as "skip"
/// rather than an error, to avoid blocking the student's payout.
fn try_swap_for_xlm(env: &Env, token: &Address, amount: i128) -> Option<i128> {
    // Look up the registered AMM address. If none is configured, skip.
    let amm_address: Option<Address> = env.storage().instance().get(&DataKey::ApprovedAmm(
        // Use the token address as the AMM lookup key (one AMM per token).
        token.clone(),
    ));

    let amm = amm_address?;

    // Invoke the AMM's `swap` function: swap `amount` of `token` for XLM.
    // The AMM contract is expected to implement:
    //   fn swap(token_in: Address, amount_in: i128, min_amount_out: i128) -> i128
    //
    // We set min_amount_out = 1 stroop to accept any non-zero return, since
    // the deduction is already micro-sized and we prioritise not blocking the
    // student's payout over getting a fair exchange rate.
    let result = env.try_invoke_contract::<i128>(
        &amm,
        &Symbol::new(env, "swap"),
        soroban_sdk::vec![
            env,
            token.clone().into_val(env),
            amount.into_val(env),
            1i128.into_val(env), // min_amount_out = 1 stroop
        ],
    );

    match result {
        Ok(Ok(xlm_out)) if xlm_out > 0 => Some(xlm_out),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// View helper
// ---------------------------------------------------------------------------

/// Returns the timestamp of the last automatic rent extension, or 0 if the
/// hook has never fired. Useful for off-chain monitoring.
pub fn last_rent_extended(env: &Env) -> u64 {
    env.storage()
        .instance()
        .get(&DataKey::RentLastExtended)
        .unwrap_or(0)
}
