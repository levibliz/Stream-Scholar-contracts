// Soroban-native safe arithmetic helpers.
//
// Each helper wraps the host-provided `checked_*` op for the integer width
// it operates on and traps the contract with a structured `MathErr` code
// when the operation would overflow, underflow, or divide by zero. Routing
// every arithmetic site through this module gives every monetary update a
// uniform failure mode that is observable from RPC clients (no opaque
// arithmetic-overflow VM trap) and lets us add coverage in one place.

use crate::MathErr;
use soroban_sdk::Env;

#[inline]
pub fn add_i128(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_add(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[inline]
pub fn sub_i128(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_sub(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Underflow))
}

#[inline]
pub fn mul_i128(env: &Env, a: i128, b: i128) -> i128 {
    a.checked_mul(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[inline]
pub fn div_i128(env: &Env, a: i128, b: i128) -> i128 {
    if b == 0 {
        env.panic_with_error(MathErr::DivisionByZero);
    }
    a.checked_div(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[allow(dead_code)]
#[inline]
pub fn add_u128(env: &Env, a: u128, b: u128) -> u128 {
    a.checked_add(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[inline]
pub fn add_u64(env: &Env, a: u64, b: u64) -> u64 {
    a.checked_add(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[inline]
pub fn sub_u64(env: &Env, a: u64, b: u64) -> u64 {
    a.checked_sub(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Underflow))
}

#[inline]
pub fn mul_u64(env: &Env, a: u64, b: u64) -> u64 {
    a.checked_mul(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[inline]
pub fn add_u32(env: &Env, a: u32, b: u32) -> u32 {
    a.checked_add(b)
        .unwrap_or_else(|| env.panic_with_error(MathErr::Overflow))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_i128_normal_path() {
        let env = Env::default();
        assert_eq!(add_i128(&env, 1, 2), 3);
        assert_eq!(add_i128(&env, i128::MAX - 1, 1), i128::MAX);
    }

    #[test]
    #[should_panic]
    fn add_i128_overflow_traps() {
        let env = Env::default();
        let _ = add_i128(&env, i128::MAX, 1);
    }

    #[test]
    fn sub_i128_normal_path() {
        let env = Env::default();
        assert_eq!(sub_i128(&env, 5, 3), 2);
        assert_eq!(sub_i128(&env, i128::MIN + 1, 1), i128::MIN);
    }

    #[test]
    #[should_panic]
    fn sub_i128_underflow_traps() {
        let env = Env::default();
        let _ = sub_i128(&env, i128::MIN, 1);
    }

    #[test]
    fn mul_i128_normal_path() {
        let env = Env::default();
        assert_eq!(mul_i128(&env, 3, 4), 12);
        assert_eq!(mul_i128(&env, -2, 3), -6);
    }

    #[test]
    #[should_panic]
    fn mul_i128_overflow_traps() {
        let env = Env::default();
        let _ = mul_i128(&env, i128::MAX, 2);
    }

    #[test]
    fn div_i128_normal_path() {
        let env = Env::default();
        assert_eq!(div_i128(&env, 10, 2), 5);
        assert_eq!(div_i128(&env, -10, 2), -5);
    }

    #[test]
    #[should_panic]
    fn div_i128_by_zero_traps() {
        let env = Env::default();
        let _ = div_i128(&env, 1, 0);
    }

    #[test]
    #[should_panic]
    fn div_i128_overflow_traps() {
        // i128::MIN / -1 overflows because |i128::MIN| > i128::MAX.
        let env = Env::default();
        let _ = div_i128(&env, i128::MIN, -1);
    }

    #[test]
    fn add_u64_normal_path() {
        let env = Env::default();
        assert_eq!(add_u64(&env, 100, 200), 300);
    }

    #[test]
    #[should_panic]
    fn add_u64_overflow_traps() {
        let env = Env::default();
        let _ = add_u64(&env, u64::MAX, 1);
    }

    #[test]
    #[should_panic]
    fn sub_u64_underflow_traps() {
        let env = Env::default();
        let _ = sub_u64(&env, 5, 10);
    }

    #[test]
    #[should_panic]
    fn mul_u64_overflow_traps() {
        let env = Env::default();
        let _ = mul_u64(&env, u64::MAX, 2);
    }

    #[test]
    #[should_panic]
    fn add_u32_overflow_traps() {
        let env = Env::default();
        let _ = add_u32(&env, u32::MAX, 1);
    }

    #[test]
    #[should_panic]
    fn add_u128_overflow_traps() {
        let env = Env::default();
        let _ = add_u128(&env, u128::MAX, 1);
    }
}
