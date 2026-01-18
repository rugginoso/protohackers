pub fn is_prime(n: u64) -> bool {
    if n <= 1 {
        return false;
    }
    if n <= 3 {
        return true;
    }

    if n.is_multiple_of(2) || n.is_multiple_of(3) {
        return false;
    }

    let mut i = 5;
    while i * i <= n {
        if n.is_multiple_of(i) || n.is_multiple_of(i + 2) {
            return false;
        }
        i += 6;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::is_prime;

    #[test]
    fn primes_edge_cases() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
    }

    #[test]
    fn primes_small_numbers() {
        let primes = [5, 7, 11, 13, 17, 19, 23, 29];
        let non_primes = [4, 6, 8, 9, 10, 12, 14, 15, 16, 18];

        for p in primes {
            assert!(is_prime(p), "{} should be prime", p);
        }

        for n in non_primes {
            assert!(!is_prime(n), "{} should not be prime", n);
        }
    }

    #[test]
    fn primes_even_and_multiple_of_three() {
        for n in (4..1000).step_by(2) {
            assert!(!is_prime(n), "{} is even", n);
        }

        for n in (9..1000).step_by(3) {
            assert!(!is_prime(n), "{} is multiple of 3", n);
        }
    }

    #[test]
    fn primes_perfect_squares() {
        let squares = [4, 9, 25, 49, 121, 169, 289];

        for n in squares {
            assert!(!is_prime(n), "{} is a perfect square", n);
        }
    }

    #[test]
    fn primes_large_known() {
        let primes = [
            1_000_000_007,
            1_000_000_009,
            4_294_967_291, // primo < 2^32
        ];

        for p in primes {
            assert!(is_prime(p), "{} should be prime", p);
        }
    }

    #[test]
    fn primes_semiprimes() {
        let cases = [
            77,                    // 7 * 11
            143,                   // 11 * 13
            1_000_003 * 1_000_033, // semiprimo grande
        ];

        for n in cases {
            assert!(!is_prime(n), "{} is composite", n);
        }
    }

    #[test]
    fn primes_u64_limits() {
        assert!(!is_prime(u64::MAX));
        assert!(!is_prime(u64::MAX - 1));
    }

    #[test]
    fn primes_no_false_positives_small_range() {
        for n in 2..10_000 {
            if is_prime(n) {
                for d in 2..=((n as f64).sqrt() as u64) {
                    assert!(n % d != 0, "{} reported prime but divisible by {}", n, d);
                }
            }
        }
    }
}
