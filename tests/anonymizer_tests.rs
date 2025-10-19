#[cfg(test)]
mod tests {
    use membench::record::Anonymizer;

    #[test]
    fn test_hash_deterministic() {
        let anon = Anonymizer::new(12345);

        let hash1 = anon.hash_key(b"testkey");
        let hash2 = anon.hash_key(b"testkey");

        assert_eq!(hash1, hash2, "hashing same key should produce same hash");
    }

    #[test]
    fn test_different_keys_different_hashes() {
        let anon = Anonymizer::new(12345);

        let hash1 = anon.hash_key(b"key1");
        let hash2 = anon.hash_key(b"key2");

        assert_ne!(
            hash1, hash2,
            "different keys should produce different hashes"
        );
    }

    #[test]
    fn test_different_salts_different_hashes() {
        let anon1 = Anonymizer::new(12345);
        let anon2 = Anonymizer::new(54321);

        let hash1 = anon1.hash_key(b"testkey");
        let hash2 = anon2.hash_key(b"testkey");

        assert_ne!(
            hash1, hash2,
            "different salts should produce different hashes"
        );
    }
}
