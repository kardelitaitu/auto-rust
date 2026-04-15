use rand::Rng;
use rand_distr::{Distribution, Normal};

#[allow(dead_code)]
pub fn random_in_range(min: u64, max: u64) -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen_range(min..=max)
}

#[allow(dead_code)]
pub fn gaussian(mean: f64, std_dev: f64, min: f64, max: f64) -> f64 {
    let normal = Normal::new(mean, std_dev).unwrap();
    let mut rng = rand::thread_rng();

    loop {
        let sample = normal.sample(&mut rng);
        if sample >= min && sample <= max {
            return sample;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn test_random_in_range() {
        let result = random_in_range(10, 20);
        assert!(result >= 10 && result <= 20);
    }

    #[test]
    fn test_gaussian_bounds() {
        // Test that gaussian respects bounds
        let result = gaussian(100.0, 10.0, 80.0, 120.0);
        assert!(result >= 80.0 && result <= 120.0);
    }

    #[test]
    fn test_gaussian_mean() {
        // Test that gaussian centers around mean (with some tolerance)
        // This is a statistical test - run multiple times
        let mut sum = 0.0;
        let samples = 1000;

        for _ in 0..samples {
            sum += gaussian(50.0, 5.0, 0.0, 100.0);
        }

        let mean = sum / samples as f64;
        // Should be close to 50.0 (within 2 standard deviations)
        assert!((mean - 50.0).abs() < 10.0);
    }

    #[test]
    fn test_gaussian_standard_deviation() {
        // Test that standard deviation is reasonable
        let mut values = Vec::new();
        let samples = 10000;
        let expected_mean = 100.0;
        let expected_std = 15.0;

        for _ in 0..samples {
            values.push(gaussian(expected_mean, expected_std, 0.0, 200.0));
        }

        let mean = values.iter().sum::<f64>() / samples as f64;
        let variance = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples as f64;
        let std_dev = variance.sqrt();

        // Should be close to expected standard deviation (within 20% tolerance)
        assert!((std_dev - expected_std).abs() / expected_std < 0.2);
    }
}
