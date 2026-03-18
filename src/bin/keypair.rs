use iroh::SecretKey;

fn main() {
    let secret_key = SecretKey::generate(&mut rand::rng());
    dbg!(secret_key.to_bytes());
}
