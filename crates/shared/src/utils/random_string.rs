use anyhow::Result;
use rand::rngs::{OsRng, StdRng};
use rand::{Rng, SeedableRng, TryRngCore};

const CHARACTERS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

pub fn generate_random_string(length: usize) -> Result<String> {
    let mut seed = [0u8; 32];
    OsRng.try_fill_bytes(&mut seed)?;
    let mut rng = StdRng::from_seed(seed);

    let s = (0..length)
        .map(|_| {
            let idx = rng.random_range(0..CHARACTERS.len());
            CHARACTERS[idx] as char
        })
        .collect();

    Ok(s)
}
