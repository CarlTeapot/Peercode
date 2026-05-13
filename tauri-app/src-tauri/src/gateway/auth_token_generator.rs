use rand::rngs::OsRng;
use rand::RngCore;

pub fn generate_gateway_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    hex::encode(bytes)
}
