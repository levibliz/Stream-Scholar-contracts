// Pure (no_std, no Soroban) helpers for the partial-claim and rounding paths
// of the Stream-Scholar contract. The functions here mirror the arithmetic
// performed inline in `scholar_contracts::ScholarContract` — extracting it
// keeps the math fuzz-targetable without spinning up a Soroban Env, and gives
// us a single place to encode the invariants the contract relies on.
//
// Every helper returns either a checked `Option`/`Result` value or a plain
// `i128`/`u64`. None of them silently saturate; integer overflow is reported
// to the caller, who can decide whether to panic, skip, or surface it as an
// error. The contract's existing `safe_math` wrappers translate `None` into a
// `MathErr` panic with a structured error code at the call sites.

#![no_std]

/// Final-release lock percentage: the contract holds back the last 10% of the
/// total grant until the community vote passes.
pub const FINAL_RELEASE_PERCENTAGE: i128 = 10;
/// Scaling factor for tax expressed in basis points (1bps = 0.01%).
pub const BPS_DENOMINATOR: i128 = 10_000;
/// Scaling factor for percent-based math.
pub const PERCENT_DENOMINATOR: i128 = 100;
/// Mirror of `NATIVE_XLM_RESERVE` in the contract: 2 XLM kept aside as gas.
pub const NATIVE_XLM_RESERVE: i128 = 2_0000000;

/// Reasons a partial claim can be rejected. These intentionally mirror the
/// `panic!` strings used inline in `withdraw_scholarship` so the fuzz target
/// can recreate the contract's accept/reject decision tree exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimError {
    /// Requested amount is non-positive.
    InvalidAmount,
    /// Final 10% is locked and the community vote has not passed.
    FinalReleaseLocked,
    /// Amount exceeds the unlocked-and-not-locked balance available.
    ExceedsAvailable,
    /// Scholarship balance is below the requested amount.
    InsufficientBalance,
    /// An intermediate calculation overflowed i128.
    Overflow,
}

/// Outcome of a successful partial-claim simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialClaim {
    pub gross_amount: i128,
    pub tax_amount: i128,
    pub net_amount: i128,
    pub new_balance: i128,
    pub new_unlocked_balance: i128,
}

/// Outcome of a stateless `simulate_claim` view; matches the contract's
/// `ClaimSimulation` struct minus the gas-fee field that is a constant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulatedClaim {
    pub tokens_to_release: i128,
    pub tax_withholding_amount: i128,
    pub net_claimable_amount: i128,
}

/// `(scholarship.total_grant * 10) / 100`, i.e. the locked portion the
/// community-vote final-release path covers. Returns `None` on overflow.
#[inline]
pub fn final_release_locked_amount(total_grant: i128) -> Option<i128> {
    if total_grant <= 0 {
        return Some(0);
    }
    total_grant
        .checked_mul(FINAL_RELEASE_PERCENTAGE)
        .map(|p| p / PERCENT_DENOMINATOR)
}

/// Computes the available-to-withdraw amount for a partial claim: starts from
/// `unlocked_balance` then clamps it down so the locked 10% is never crossed.
#[inline]
pub fn available_to_withdraw(
    unlocked_balance: i128,
    balance: i128,
    total_grant: i128,
    final_release_claimed: bool,
) -> Option<i128> {
    let mut available = unlocked_balance.max(0);
    if !final_release_claimed && total_grant > 0 {
        let locked = final_release_locked_amount(total_grant)?;
        if balance > locked {
            let cap = balance.checked_sub(locked)?;
            if cap < available {
                available = cap;
            }
        } else {
            available = 0;
        }
    }
    Some(available)
}

/// Applies basis-point tax to a gross amount; returns `(net, tax)` where
/// `net + tax == amount`.
///
/// The contract floors the tax at `(amount * bps) / 10_000`, which means the
/// fractional remainder is silently kept by the student (bias toward the
/// student). The fuzz target asserts the no-value-lost invariant.
#[inline]
pub fn apply_bps_tax(amount: i128, tax_bps: u32) -> Option<(i128, i128)> {
    if tax_bps > BPS_DENOMINATOR as u32 {
        return None;
    }
    let bps = tax_bps as i128;
    let tax = amount.checked_mul(bps)?.checked_div(BPS_DENOMINATOR)?;
    let net = amount.checked_sub(tax)?;
    Some((net, tax))
}

/// Replicates the `simulate_claim` view from the contract. Returns the same
/// values the on-chain function would return for a given scholarship state.
///
/// `is_native` plus `NATIVE_XLM_RESERVE` enforces the 2-XLM gas reserve on
/// native-asset scholarships.
#[inline]
pub fn simulate_partial_claim(
    unlocked_balance: i128,
    balance: i128,
    total_grant: i128,
    final_release_claimed: bool,
    is_native: bool,
    tax_bps: u32,
) -> Option<SimulatedClaim> {
    let mut tokens_to_release = unlocked_balance.max(0);

    if !final_release_claimed && total_grant > 0 {
        let locked = final_release_locked_amount(total_grant)?;
        if balance > locked {
            let cap = balance.checked_sub(locked)?;
            if cap < tokens_to_release {
                tokens_to_release = cap;
            }
        } else {
            tokens_to_release = 0;
        }
    }

    if is_native {
        if balance > NATIVE_XLM_RESERVE {
            let cap = balance.checked_sub(NATIVE_XLM_RESERVE)?;
            if cap < tokens_to_release {
                tokens_to_release = cap;
            }
        } else {
            tokens_to_release = 0;
        }
    }

    let (net, tax) = apply_bps_tax(tokens_to_release, tax_bps)?;
    Some(SimulatedClaim {
        tokens_to_release,
        tax_withholding_amount: tax,
        net_claimable_amount: net,
    })
}

/// Validates and applies a partial-claim withdrawal request. This mirrors the
/// inline accept/reject logic in `withdraw_scholarship`. On success the caller
/// gets back the post-withdrawal balances and the (net, tax) breakdown.
pub fn execute_partial_claim(
    unlocked_balance: i128,
    balance: i128,
    total_grant: i128,
    final_release_claimed: bool,
    requested: i128,
    tax_bps: u32,
) -> Result<PartialClaim, ClaimError> {
    if requested <= 0 {
        return Err(ClaimError::InvalidAmount);
    }
    let locked = final_release_locked_amount(total_grant).ok_or(ClaimError::Overflow)?;
    if balance <= locked && !final_release_claimed {
        return Err(ClaimError::FinalReleaseLocked);
    }
    let available = available_to_withdraw(unlocked_balance, balance, total_grant, final_release_claimed)
        .ok_or(ClaimError::Overflow)?;
    if requested > available {
        return Err(ClaimError::ExceedsAvailable);
    }
    if balance < requested {
        return Err(ClaimError::InsufficientBalance);
    }
    let (net, tax) = apply_bps_tax(requested, tax_bps).ok_or(ClaimError::Overflow)?;
    let new_balance = balance.checked_sub(requested).ok_or(ClaimError::Overflow)?;
    let new_unlocked = unlocked_balance
        .checked_sub(requested)
        .ok_or(ClaimError::Overflow)?;
    Ok(PartialClaim {
        gross_amount: requested,
        tax_amount: tax,
        net_amount: net,
        new_balance,
        new_unlocked_balance: new_unlocked,
    })
}

/// 70/30 (university/student) tuition split. Floors the university share so
/// dust remains with the student. Returns `(university, student)` where
/// `university + student == amount` for any non-negative `amount`.
#[inline]
pub fn tuition_split(amount: i128, university_pct: u32) -> Option<(i128, i128)> {
    if university_pct > 100 || amount < 0 {
        return None;
    }
    let pct = university_pct as i128;
    let university = amount.checked_mul(pct)?.checked_div(PERCENT_DENOMINATOR)?;
    let student = amount.checked_sub(university)?;
    Some((university, student))
}

/// Clawback amount: `(balance * pct) / 100`, with `pct` capped at 100. Floors,
/// so the funder never claws back more than their entitlement.
#[inline]
pub fn clawback_amount(balance: i128, percent: u64) -> Option<i128> {
    if percent > 100 || balance < 0 {
        return None;
    }
    let pct = percent as i128;
    balance.checked_mul(pct)?.checked_div(PERCENT_DENOMINATOR)
}

/// Discounted streaming rate: `(rate * (100 - discount_pct)) / 100` expressed
/// in the contract's `(rate * pct)/100` shape. Floors the discount, so the
/// effective rate is always slightly higher than the floating-point ideal.
#[inline]
pub fn discount_rate(rate: i128, discount_pct: u32) -> Option<i128> {
    if discount_pct > 100 || rate < 0 {
        return None;
    }
    let pct = discount_pct as i128;
    let discount = rate.checked_mul(pct)?.checked_div(PERCENT_DENOMINATOR)?;
    rate.checked_sub(discount)
}

/// GPA multiplier in basis points (e.g. 12_000 = 1.2x). Returns the rate
/// scaled by `multiplier_bps / 10_000`.
#[inline]
pub fn gpa_multiplied_rate(rate: i128, multiplier_bps: u64) -> Option<i128> {
    let mul = multiplier_bps as i128;
    rate.checked_mul(mul)?.checked_div(BPS_DENOMINATOR)
}

/// Per-project quadratic-funding match: `(Σ√c)² − Σc`, clamped at zero. The
/// inline contract code uses `.max(0)` after subtraction so a project that
/// raised more than the square sum gets no negative match.
#[inline]
pub fn qf_matching_for_project(sqrt_sum_contributions: i128, total_raised: i128) -> Option<i128> {
    let square = sqrt_sum_contributions.checked_mul(sqrt_sum_contributions)?;
    let diff = square.checked_sub(total_raised)?;
    Some(diff.max(0))
}

/// Newton-iteration integer square root over i128. Returns the floor of √n
/// for non-negative `n`, and 0 for negative inputs (matching the contract).
pub fn isqrt(n: i128) -> i128 {
    if n <= 0 {
        return 0;
    }
    let mut x = n;
    let mut y = match x.checked_add(1) {
        Some(s) => s / 2,
        None => x / 2,
    };
    while y < x {
        x = y;
        let step = n / x;
        y = match x.checked_add(step) {
            Some(s) => s / 2,
            None => x,
        };
    }
    x
}

/// Pay-It-Forward alumni-tax accumulator. The contract tracks fractional
/// remainders ("dust") in storage and rolls them into the next tax cycle so
/// long-tail rounding loss is eventually paid out.
///
/// Returns `(amount_remaining_to_alumni, new_dust, tax_amount)` where
/// `amount_remaining_to_alumni + tax_amount == amount` for the alumni's
/// portion, and `new_dust < 100` for any non-pathological input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlumniTaxResult {
    pub amount_to_alumni: i128,
    pub tax_amount: i128,
    pub new_dust: i128,
}

#[inline]
pub fn apply_alumni_tax(amount: i128, percentage: u32, current_dust: i128) -> Option<AlumniTaxResult> {
    if percentage > 100 || amount < 0 || current_dust < 0 {
        return None;
    }
    let pct = percentage as i128;
    let raw_tax = amount.checked_mul(pct)?;
    let mut tax = raw_tax / PERCENT_DENOMINATOR;
    let dust = raw_tax % PERCENT_DENOMINATOR;
    let mut new_dust = current_dust.checked_add(dust)?;
    if new_dust >= PERCENT_DENOMINATOR {
        tax = tax.checked_add(new_dust / PERCENT_DENOMINATOR)?;
        new_dust %= PERCENT_DENOMINATOR;
    }
    let to_alumni = if tax > 0 {
        amount.checked_sub(tax)?
    } else {
        amount
    };
    Some(AlumniTaxResult {
        amount_to_alumni: to_alumni,
        tax_amount: tax,
        new_dust,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locked_amount_is_ten_percent() {
        assert_eq!(final_release_locked_amount(0), Some(0));
        assert_eq!(final_release_locked_amount(100), Some(10));
        assert_eq!(final_release_locked_amount(99), Some(9));
        assert_eq!(final_release_locked_amount(1), Some(0));
    }

    #[test]
    fn locked_amount_overflow() {
        assert_eq!(final_release_locked_amount(i128::MAX), None);
    }

    #[test]
    fn tax_no_value_lost() {
        for amt in [0i128, 1, 100, 1_000, 10_000_000_000] {
            for bps in [0u32, 1, 100, 1_000, 9_999, 10_000] {
                let (net, tax) = apply_bps_tax(amt, bps).unwrap();
                assert_eq!(net + tax, amt, "amt={} bps={}", amt, bps);
                assert!(tax >= 0 && net >= 0);
                assert!(tax <= amt);
            }
        }
    }

    #[test]
    fn tuition_split_sums_to_amount() {
        for amt in [0i128, 1, 33, 100, 999, 10_000] {
            for pct in [0u32, 1, 30, 50, 70, 99, 100] {
                let (u, s) = tuition_split(amt, pct).unwrap();
                assert_eq!(u + s, amt, "amt={} pct={}", amt, pct);
            }
        }
    }

    #[test]
    fn isqrt_floor_property() {
        for n in [0i128, 1, 2, 3, 4, 9, 10, 99, 100, 10_001, 1_000_000] {
            let r = isqrt(n);
            assert!(r >= 0);
            assert!(r * r <= n);
            assert!((r + 1).saturating_mul(r + 1) > n || r == i128::MAX);
        }
    }

    #[test]
    fn partial_claim_rejects_zero_amount() {
        let err = execute_partial_claim(100, 100, 100, false, 0, 0).unwrap_err();
        assert_eq!(err, ClaimError::InvalidAmount);
    }

    #[test]
    fn partial_claim_blocks_locked_window() {
        // total_grant=100, locked=10, balance=10 (= locked) → blocked
        let err = execute_partial_claim(10, 10, 100, false, 5, 0).unwrap_err();
        assert_eq!(err, ClaimError::FinalReleaseLocked);
    }

    #[test]
    fn partial_claim_full_path_after_unlock() {
        // Final release claimed → locked window is bypassed.
        let r = execute_partial_claim(50, 50, 100, true, 50, 0).unwrap();
        assert_eq!(r.new_balance, 0);
        assert_eq!(r.new_unlocked_balance, 0);
        assert_eq!(r.tax_amount, 0);
        assert_eq!(r.net_amount, 50);
    }

    #[test]
    fn partial_claim_respects_locked_amount() {
        // total_grant=1000, locked=100, balance=500, unlocked=500.
        // available = min(unlocked=500, balance-locked=400) = 400.
        // Requesting 401 should fail.
        let err = execute_partial_claim(500, 500, 1000, false, 401, 0).unwrap_err();
        assert_eq!(err, ClaimError::ExceedsAvailable);
        // Requesting exactly 400 should succeed.
        let r = execute_partial_claim(500, 500, 1000, false, 400, 0).unwrap();
        assert_eq!(r.new_balance, 100);
        assert_eq!(r.new_unlocked_balance, 100);
    }

    #[test]
    fn alumni_tax_dust_rollover() {
        // 7% of 13 = 0.91, so raw_tax = 91, dust = 91, tax = 0.
        let r = apply_alumni_tax(13, 7, 0).unwrap();
        assert_eq!(r.tax_amount, 0);
        assert_eq!(r.new_dust, 91);
        assert_eq!(r.amount_to_alumni, 13);
        // Next call with the rolled dust should pay out 1 unit and reset.
        let r2 = apply_alumni_tax(13, 7, r.new_dust).unwrap();
        assert_eq!(r2.tax_amount, 1);
        assert_eq!(r2.new_dust, 82);
        assert_eq!(r2.amount_to_alumni, 12);
    }

    #[test]
    fn qf_matching_clamps_negative() {
        // sqrt_sum=10 → square=100; raised=200 → diff=-100 → clamp to 0.
        assert_eq!(qf_matching_for_project(10, 200), Some(0));
        assert_eq!(qf_matching_for_project(10, 50), Some(50));
    }

    #[test]
    fn simulate_partial_claim_native_reserve() {
        // is_native=true with balance just above the 2 XLM reserve floor.
        let s = simulate_partial_claim(
            10_0000000,
            10_0000000,
            0, // total_grant=0 disables locked-window
            true,
            true,
            0,
        )
        .unwrap();
        // Releasable = balance - reserve = 8 XLM.
        assert_eq!(s.tokens_to_release, 10_0000000 - NATIVE_XLM_RESERVE);
    }
}
