//! A dependency-free fixture crate: the smallest target Flow can verify.
//!
//! The public surface is one integer addition plus its unit tests. It is
//! deliberately trivial: the fixture exists to exercise the verification path
//! (format, lint, test), not to be useful on its own.

/// Add two integers.
///
/// # Examples
///
/// ```
/// assert_eq!(polyglot_rust::add(2, 3), 5);
/// ```
pub fn add(left: i64, right: i64) -> i64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_positive_numbers() {
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn adds_negative_numbers() {
        assert_eq!(add(-2, 3), 1);
    }
}
