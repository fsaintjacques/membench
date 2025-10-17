use siphasher::sip::SipHasher13;
use std::hash::{Hash, Hasher};

pub struct Anonymizer {
    salt: u64,
}

impl Anonymizer {
    pub fn new(salt: u64) -> Self {
        Anonymizer { salt }
    }

    pub fn hash_key(&self, key: &[u8]) -> u64 {
        let key_bytes = self.salt.to_le_bytes();
        let mut hasher_key = [0u8; 16];
        hasher_key[0..8].copy_from_slice(&key_bytes);
        hasher_key[8..16].copy_from_slice(&key_bytes);

        let mut hasher = SipHasher13::new_with_key(&hasher_key);
        key.hash(&mut hasher);
        hasher.finish()
    }
}
