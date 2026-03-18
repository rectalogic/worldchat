use rand::RngCore;

fn main() {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    dbg!(bytes);
}
