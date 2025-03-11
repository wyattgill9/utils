use num_bigint::BigInt;

pub fn fib(n: isize) -> BigInt {
    fib_luc(n).0
}

fn fib_luc(mut n: isize) -> (BigInt, BigInt) {
    if n == 0 {
        return (BigInt::ZERO, BigInt::from(2));
    }

    if n < 0 {
        n = -n;
        let (fib, luc) = fib_luc(n);
        let k = if n % 2 == 0 { 1 } else { -1 };
        return (fib * k, luc * k);
    }

    if n & 1 == 1 {
        let (fib, luc) = fib_luc(n - 1);
        return ((&fib + &luc) >> 1, (BigInt::from(5) * &fib + &luc) >> 1);
    }

    n >>= 1;
    let k = (n % 2) * 2 - 1;
    let (fib, luc) = fib_luc(n);
    (&fib * &luc, &luc * &luc + BigInt::from(2) * k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigInt;

    #[test]
    fn test_base_cases() {
        assert_eq!(fib_luc(0).0, BigInt::from(0));
        assert_eq!(fib_luc(1).0, BigInt::from(1));
        assert_eq!(fib_luc(2).0, BigInt::from(1));
    }

    #[test]
    fn test_small_values() {
        assert_eq!(fib_luc(3).0, BigInt::from(2));
        assert_eq!(fib_luc(4).0, BigInt::from(3));
        assert_eq!(fib_luc(5).0, BigInt::from(5));
    }

    #[test]
    fn test_larger_values() {
        assert_eq!(fib_luc(10).0, BigInt::from(55));
        assert_eq!(fib_luc(15).0, BigInt::from(610));
        assert_eq!(fib_luc(20).0, BigInt::from(6765));
    }
}