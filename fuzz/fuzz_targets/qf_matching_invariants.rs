// Fuzz the quadratic-funding match calculation and the integer square root
// it depends on. Properties we enforce:
//
//   1. `isqrt(n)` is the floor of √n: `isqrt(n)² <= n < (isqrt(n)+1)²` on
//      the safe range. Negative inputs return 0 (matches the contract).
//   2. `qf_matching_for_project(s, raised) >= 0` always — the contract
//      clamps negative differences to 0 so a project that raised more than
//      the square of its sqrt-sum gets no match.
//   3. Across an arbitrary slate of projects, summing per-project matches
//      never exceeds `(Σ s)²` (this is weaker than the on-chain budget
//      check, which uses `(Σ s)² − Σraised`, but it's the right invariant
//      for the per-project clamp path).
//   4. A project that contributed zero (sqrt_sum = 0) gets zero match.

#![no_main]

use arbitrary::Arbitrary;
use claim_math::{isqrt, qf_matching_for_project};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Project {
    contribution_anchor: u8,
    contribution_offset: i64,
    raised_anchor: u8,
    raised_offset: i64,
}

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    isqrt_seed: i64,
    isqrt_anchor: u8,
    project_count: u8,
    projects: [Project; 8],
}

/// Anchor a square-root input toward boundary cases: 0, perfect squares,
/// off-by-one from a perfect square, and large values that still leave room
/// for `(isqrt(n)+1)²` not to overflow.
fn anchor_isqrt(anchor: u8, seed: i64) -> i128 {
    match anchor % 8 {
        0 => 0,
        1 => 1,
        2 => 4,
        3 => 100,
        4 => 9_999, // one short of 100²
        5 => 10_000,
        6 => (seed.unsigned_abs() as i128).saturating_mul(seed.unsigned_abs() as i128),
        _ => seed as i128,
    }
}

/// Anchor a per-project contribution to small, medium, and large values.
/// Capped well below i128::MAX so squaring stays in range during the
/// per-project match computation.
fn anchor_contribution(anchor: u8, offset: i64) -> i128 {
    let v = match anchor % 6 {
        0 => 0,
        1 => 1,
        2 => 100,
        3 => 1_000,
        4 => offset.unsigned_abs() as i128 % 1_000_000,
        _ => offset.unsigned_abs() as i128 % 1_000_000_000,
    };
    v.max(0)
}

fuzz_target!(|input: FuzzInput| {
    // ── isqrt floor property ─────────────────────────────────────────────
    let n = anchor_isqrt(input.isqrt_anchor, input.isqrt_seed);
    let r = isqrt(n);

    if n <= 0 {
        assert_eq!(r, 0);
    } else {
        // r² <= n.
        if let Some(sq) = r.checked_mul(r) {
            assert!(sq <= n, "isqrt({}) = {} but {}² = {} > n", n, r, r, sq);
        }
        // (r+1)² > n, modulo the i128::MAX overflow-corner case.
        if let Some(next) = r.checked_add(1) {
            if let Some(next_sq) = next.checked_mul(next) {
                assert!(
                    next_sq > n,
                    "isqrt({}) = {} but ({}+1)² = {} <= n",
                    n,
                    r,
                    r,
                    next_sq
                );
            }
        }
    }

    // ── Per-project QF match clamp ───────────────────────────────────────
    let count = ((input.project_count as usize) % 8) + 1;
    let mut sum_sqrt: i128 = 0;
    let mut sum_match: i128 = 0;
    let mut overflowed = false;

    for i in 0..count {
        let p = &input.projects[i];
        let raw = anchor_contribution(p.contribution_anchor, p.contribution_offset);
        let raised = anchor_contribution(p.raised_anchor, p.raised_offset);
        let s = isqrt(raw);

        // Match is non-negative.
        match qf_matching_for_project(s, raised) {
            Some(m) => {
                assert!(m >= 0, "match went negative for s={} raised={}", s, raised);
                if s == 0 {
                    assert_eq!(m, 0, "zero-contribution project earned match");
                }
                if let Some(next) = sum_match.checked_add(m) {
                    sum_match = next;
                } else {
                    overflowed = true;
                    break;
                }
            }
            None => {
                overflowed = true;
                break;
            }
        }
        if let Some(next) = sum_sqrt.checked_add(s) {
            sum_sqrt = next;
        } else {
            overflowed = true;
            break;
        }
    }

    // The aggregate cap: per-project matches sum to at most (Σ√c)². Each
    // project's match is `s² − raised` clamped at 0, so `Σ matches ≤ Σ s²`,
    // and `Σ s² ≤ (Σ s)²` for non-negative s.
    if !overflowed {
        if let Some(sq) = sum_sqrt.checked_mul(sum_sqrt) {
            assert!(
                sum_match <= sq,
                "Σ matches={} exceeded (Σ s)²={}",
                sum_match,
                sq
            );
        }
    }
});
