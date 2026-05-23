use std::time::Duration;

/// Compute the next retry delay using exponential backoff with jitter.
///
/// Formula: min(base * 2^attempt, max) ± 20% jitter
pub fn next_delay(attempt: u32, base_secs: u64, max_secs: u64) -> Duration {
    let exp = (base_secs as f64) * 2_f64.powi(attempt as i32);
    let capped = exp.min(max_secs as f64);
    // Add ±20% jitter using pseudo-random from attempt number (deterministic for testing)
    let jitter_factor = 0.8 + ((((attempt as u64).wrapping_mul(6364136223846793005)) % 100) as f64 / 250.0);
    let delay = capped * jitter_factor;
    Duration::from_secs_f64(delay.max(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backoff_increases() {
        let d0 = next_delay(0, 5, 300);
        let d1 = next_delay(1, 5, 300);
        let d2 = next_delay(2, 5, 300);
        // Each attempt should generally increase (allowing for jitter)
        assert!(d1.as_secs_f64() >= d0.as_secs_f64() * 0.5);
        assert!(d2.as_secs_f64() >= d1.as_secs_f64() * 0.5);
    }

    #[test]
    fn test_backoff_capped_at_max() {
        let delay = next_delay(20, 5, 300);
        assert!(delay.as_secs() <= 400); // max + 20% jitter overhead
    }

    #[test]
    fn test_minimum_delay() {
        let delay = next_delay(0, 0, 0);
        assert!(delay.as_secs() >= 1);
    }
}
