//! Minimal server contract placeholder.

/// Health status returned by the early API.
#[must_use]
pub const fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::health;

    #[test]
    fn health_is_ok() {
        assert_eq!(health(), "ok");
    }
}
