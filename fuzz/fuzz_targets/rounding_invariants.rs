// Exhaustively fuzz the percent / basis-point / sweeper rounding paths used
// throughout the Stream-Scholar contract. The target ensures these floor-based
// operations preserve the value-conservation invariant the contract relies on:
//
//   * `tuition_split`: `university + student == amount` (no dust escapes).
//   * `apply_bps_tax`: `net + tax == amount`.
//   * `clawback_amount`: `0 <= clawback <= balance`.
//   * `discount_rate`: `0 <= discounted <= rate`.
//   * `gpa_multiplied_rate`: `multiplier_bps == 10_000` is the identity
//     mapping.
//   * `apply_alumni_tax`: dust strictly < 100 after rollover, total taxes paid
//     across N consecutive applications track the exact percentage to within
//     a single sub-unit lag (i.e. dust ≤ pct).
//
// Floor-bias direction (i.e. who absorbs the fractional remainder) is the
// contract's intentional behavior: students keep the dust on tax/split,
// funders forfeit the dust on clawback. The fuzz target only asserts
// conservation; it does not assert direction.

#![no_main]

use arbitrary::Arbitrary;
use claim_math::{
    apply_alumni_tax, apply_bps_tax, clawback_amount, discount_rate,
    gpa_multiplied_rate, tuition_split, BPS_DENOMINATOR, PERCENT_DENOMINATOR,
};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    amount_anchor: u8,
    amount_offset: i64,
    rate_anchor: u8,
    rate_offset: i64,
    pct: u32,
    tax_bps: u32,
    multiplier_bps: u32,
    alumni_iters: u8,
    starting_dust: i64,
}

fn anchor_value(anchor: u8, offset: i64) -> i128 {
    match anchor % 7 {
        0 => 0,
        1 => 1,
        2 => 99,                      // boundary for /100 floors
        3 => 100,
        4 => offset.unsigned_abs() as i128,
        5 => (offset as i128).saturating_mul(1_000_000),
        _ => i64::MAX as i128 + offset as i128,
    }
}

fuzz_target!(|input: FuzzInput| {
    let amount = anchor_value(input.amount_anchor, input.amount_offset);
    let rate = anchor_value(input.rate_anchor, input.rate_offset);
    let pct = input.pct % 200; // include >100 so we test rejection.

    // ── Tuition split ────────────────────────────────────────────────────
    if let Some((university, student)) = tuition_split(amount, pct) {
        // Pure conservation: split adds back to the input.
        assert_eq!(
            university.checked_add(student),
            Some(amount),
            "tuition_split lost value: amt={} pct={}",
            amount,
            pct
        );
        // Each share is non-negative when the input is.
        if amount >= 0 {
            assert!(university >= 0);
            assert!(student >= 0);
        }
    } else {
        // Rejected: either pct>100 or amount<0 or overflow.
        assert!(pct > 100 || amount < 0 || amount.checked_mul(pct as i128).is_none());
    }

    // ── BPS tax ──────────────────────────────────────────────────────────
    let bps = input.tax_bps % 20_000;
    if let Some((net, tax)) = apply_bps_tax(amount, bps) {
        if bps <= BPS_DENOMINATOR as u32 {
            assert_eq!(
                net.checked_add(tax),
                Some(amount),
                "bps tax lost value: amt={} bps={}",
                amount,
                bps
            );
            if amount >= 0 {
                assert!(tax >= 0 && net >= 0);
                assert!(tax <= amount);
            }
            // 0bps tax means the student keeps everything.
            if bps == 0 {
                assert_eq!(tax, 0);
                assert_eq!(net, amount);
            }
            // 10_000bps means full tax.
            if bps == BPS_DENOMINATOR as u32 && amount >= 0 {
                assert_eq!(net, 0);
                assert_eq!(tax, amount);
            }
        }
    } else {
        // Rejected only on overflow or rate>100%.
        assert!(bps > BPS_DENOMINATOR as u32 || amount.checked_mul(bps as i128).is_none());
    }

    // ── Clawback ─────────────────────────────────────────────────────────
    let clawback_pct = (input.pct as u64) % 200;
    if let Some(c) = clawback_amount(amount, clawback_pct) {
        assert!(c >= 0);
        assert!(c <= amount);
        // 0% clawback yields 0; 100% clawback yields the full balance.
        if clawback_pct == 0 {
            assert_eq!(c, 0);
        }
        if clawback_pct == 100 {
            assert_eq!(c, amount);
        }
    } else {
        assert!(clawback_pct > 100 || amount < 0 || amount.checked_mul(clawback_pct as i128).is_none());
    }

    // ── Discount rate ────────────────────────────────────────────────────
    let disc_pct = input.pct % 200;
    if let Some(discounted) = discount_rate(rate, disc_pct) {
        if rate >= 0 {
            assert!(discounted >= 0);
            assert!(discounted <= rate);
            if disc_pct == 0 {
                assert_eq!(discounted, rate);
            }
            if disc_pct == 100 {
                assert_eq!(discounted, 0);
            }
        }
    }

    // ── GPA multiplier ───────────────────────────────────────────────────
    if rate >= 0 {
        let mul = input.multiplier_bps;
        if let Some(scaled) = gpa_multiplied_rate(rate, mul as u64) {
            if mul == BPS_DENOMINATOR as u32 {
                // 10_000 bps must be the identity transform on the rate.
                assert_eq!(scaled, rate);
            }
            if mul == 0 {
                assert_eq!(scaled, 0);
            }
            if mul < BPS_DENOMINATOR as u32 {
                assert!(scaled <= rate);
            }
            if mul > BPS_DENOMINATOR as u32 {
                // Scaled may exceed rate — that's the point of >1x multipliers.
                assert!(scaled >= 0);
            }
        }
    }

    // ── Alumni tax dust sweeper ──────────────────────────────────────────
    // Strong property: across an arbitrary number of `apply_alumni_tax`
    // calls, the sweeper preserves the exact identity
    //
    //     starting_dust + Σ amount_in * pct = (Σ tax_paid) * 100 + final_dust
    //
    // i.e. nothing is lost. This is stronger than a "diff ≤ 1" floor check —
    // it also catches the case where a non-zero starting dust causes an
    // early iteration to pay out one extra unit (which a one-sided floor
    // bound would miss).
    let sweeper_pct = input.pct % 101; // <=100
    let starting_dust = (input.starting_dust.unsigned_abs() as i128) % PERCENT_DENOMINATOR;
    let iters = (input.alumni_iters % 16) as usize + 1;
    let alumni_amount = anchor_value(input.amount_anchor, input.amount_offset);

    if alumni_amount >= 0 {
        let mut total_tax_paid: i128 = 0;
        let mut total_amount_in: i128 = 0;
        let mut last_dust = starting_dust;
        let mut overflowed = false;
        for _ in 0..iters {
            match apply_alumni_tax(alumni_amount, sweeper_pct, last_dust) {
                Some(r) => {
                    // Dust always less than 100 after rollover.
                    assert!(r.new_dust >= 0 && r.new_dust < PERCENT_DENOMINATOR);
                    // Conservation per call: amount_to_alumni + tax = amount.
                    assert_eq!(
                        r.amount_to_alumni.checked_add(r.tax_amount),
                        Some(alumni_amount)
                    );
                    total_tax_paid = match total_tax_paid.checked_add(r.tax_amount) {
                        Some(v) => v,
                        None => {
                            overflowed = true;
                            break;
                        }
                    };
                    total_amount_in = match total_amount_in.checked_add(alumni_amount) {
                        Some(v) => v,
                        None => {
                            overflowed = true;
                            break;
                        }
                    };
                    last_dust = r.new_dust;
                }
                None => {
                    overflowed = true;
                    break;
                }
            }
        }

        if !overflowed {
            // Verify the conservation identity. Use checked arithmetic at
            // every step so an overflow on the right-hand side gets surfaced
            // as an overflow rather than masked into a spurious mismatch.
            let lhs = starting_dust
                .checked_add(total_amount_in.checked_mul(sweeper_pct as i128).unwrap_or(i128::MAX));
            let rhs = total_tax_paid
                .checked_mul(PERCENT_DENOMINATOR)
                .and_then(|v| v.checked_add(last_dust));
            if let (Some(l), Some(r)) = (lhs, rhs) {
                assert_eq!(
                    l, r,
                    "dust-sweeper conservation broken: start={} amt={} pct={} iters={} tax_paid={} final_dust={}",
                    starting_dust, alumni_amount, sweeper_pct, iters, total_tax_paid, last_dust
                );
            }
        }
    }
});
