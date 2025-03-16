use std::ops::Mul;

#[inline]
pub fn fast_power<T>(mut base: T, mut exp: usize, identity: T) -> T
where
    T: Copy + Mul<Output = T>,
{
    if exp == 0 {
        return identity;
    }

    let mut result = identity;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * base;
        }
        base = base * base;
        exp >>= 1;
    }
    result
}

pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if a == 0 {
        return (b, 0, 1);
    }

    let (gcd, x1, y1) = extended_gcd(b % a, a);
    let x = y1 - (b / a) * x1;
    let y = x1;

    (gcd, x, y)
}

pub fn mod_inverse(a: i64, m: i64) -> Option<i64> {
    let (gcd, x, _) = extended_gcd(a, m);
    if gcd != 1 {
        None // Inverse doesn't exist
    } else {
        Some(((x % m) + m) % m) // Ensure positive result
    }
}

pub fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus == 1 {
        return 0;
    }

    base %= modulus;
    let mut result = 1;

    while exp > 0 {
        if exp & 1 == 1 {
            result = (result * base) % modulus;
        }
        base = (base * base) % modulus;
        exp >>= 1;
    }

    result
}

pub fn is_prime(n: u64) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }
    if n % 2 == 0 || n % 3 == 0 {
        return false;
    }

    let witnesses = [2, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37];

    let (d, s) = factor_power_of_two(n - 1);

    for &a in witnesses.iter().take_while(|&&a| a < n) {
        let mut x = mod_pow(a, d, n);

        if x == 1 || x == n - 1 {
            continue;
        }

        let mut probably_prime = false;
        for _ in 1..s {
            x = mod_pow(x, 2, n);
            if x == n - 1 {
                probably_prime = true;
                break;
            }
        }

        if !probably_prime {
            return false;
        }
    }

    true
}

fn factor_power_of_two(n: u64) -> (u64, u64) {
    let mut d = n;
    let mut s = 0;

    while d % 2 == 0 {
        d /= 2;
        s += 1;
    }

    (d, s)
}

pub fn isqrt(n: u64) -> u64 {
    if n <= 1 {
        return n;
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    x
}

/// Prime factorization using trial division and wheel factorization
pub fn prime_factors(mut n: u64) -> Vec<u64> {
    let mut factors = Vec::new();

    while n % 2 == 0 {
        factors.push(2);
        n /= 2;
    }

    while n % 3 == 0 {
        factors.push(3);
        n /= 3;
    }

    let mut i = 5;
    let mut inc = 2;

    while i * i <= n {
        while n % i == 0 {
            factors.push(i);
            n /= i;
        }

        i += inc;
        inc = 6 - inc;
    }

    if n > 1 {
        factors.push(n);
    }

    factors
}
