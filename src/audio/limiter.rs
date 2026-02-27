//! Master limiter — hard clamp to protect output.
//!
//! Phase 0: simple hard clamp. Later phases will add lookahead with attack/release.

/// Hard limiter that clamps samples to `[-ceiling, ceiling]`.
#[derive(Debug, Clone)]
pub struct Limiter {
    ceiling: f32,
}

impl Limiter {
    /// Create a new limiter with the given ceiling (should be in `(0.0, 1.0]`).
    pub fn new(ceiling: f32) -> Self {
        debug_assert!(ceiling > 0.0 && ceiling <= 1.0);
        Self { ceiling }
    }

    /// Clamp a single sample to `[-ceiling, ceiling]`.
    #[inline]
    pub fn process(&self, sample: f32) -> f32 {
        sample.clamp(-self.ceiling, self.ceiling)
    }

    /// Clamp an entire buffer in-place.
    #[inline]
    pub fn process_block(&self, buffer: &mut [f32]) {
        for sample in buffer.iter_mut() {
            *sample = sample.clamp(-self.ceiling, self.ceiling);
        }
    }

    /// Returns the current ceiling value.
    pub fn ceiling(&self) -> f32 {
        self.ceiling
    }
}

impl Default for Limiter {
    fn default() -> Self {
        Self { ceiling: 0.95 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_limiter_passes_within_range() {
        let limiter = Limiter::new(0.95);
        assert_eq!(limiter.process(0.0), 0.0);
        assert_eq!(limiter.process(0.5), 0.5);
        assert_eq!(limiter.process(-0.5), -0.5);
        assert_eq!(limiter.process(0.95), 0.95);
        assert_eq!(limiter.process(-0.95), -0.95);
    }

    #[test]
    fn test_limiter_clamps_positive() {
        let limiter = Limiter::new(0.95);
        assert_eq!(limiter.process(1.0), 0.95);
        assert_eq!(limiter.process(2.5), 0.95);
        assert_eq!(limiter.process(f32::MAX), 0.95);
    }

    #[test]
    fn test_limiter_clamps_negative() {
        let limiter = Limiter::new(0.95);
        assert_eq!(limiter.process(-1.0), -0.95);
        assert_eq!(limiter.process(-2.5), -0.95);
        assert_eq!(limiter.process(f32::MIN), -0.95);
    }

    #[test]
    fn test_limiter_process_block() {
        let limiter = Limiter::new(0.95);
        let mut buffer = vec![0.0, 0.5, -0.5, 1.5, -1.5, 0.95, -0.95];
        let expected: Vec<f32> = buffer.iter().map(|&s| limiter.process(s)).collect();

        limiter.process_block(&mut buffer);
        assert_eq!(buffer, expected);
    }

    #[test]
    fn test_limiter_default_ceiling() {
        let limiter = Limiter::default();
        assert!((limiter.ceiling() - 0.95).abs() < f32::EPSILON);
    }

    #[test]
    fn test_limiter_custom_ceiling() {
        let limiter = Limiter::new(0.5);
        assert_eq!(limiter.process(0.6), 0.5);
        assert_eq!(limiter.process(-0.6), -0.5);
        assert_eq!(limiter.process(0.3), 0.3);
    }
}
