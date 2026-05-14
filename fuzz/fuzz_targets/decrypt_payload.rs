#![no_main]

use libfuzzer_sys::fuzz_target;
use qrd_core::encryption::{decrypt_payload, AuthTag, Nonce};
use std::convert::TryInto;

fuzz_target!(|data: &[u8]| {
    if data.len() < 60 {
        return;
    }

    let key: [u8; 32] = match data[0..32].try_into() {
        Ok(key) => key,
        Err(_) => return,
    };
    let nonce: [u8; 12] = match data[32..44].try_into() {
        Ok(nonce) => nonce,
        Err(_) => return,
    };
    let auth_tag: [u8; 16] = match data[44..60].try_into() {
        Ok(tag) => tag,
        Err(_) => return,
    };

    let _ = decrypt_payload(&data[60..], &key, &Nonce(nonce), &AuthTag(auth_tag));
});