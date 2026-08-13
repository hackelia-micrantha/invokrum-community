pub(crate) fn digest(input: &[u8]) -> [u8; 32] {
    invokrum_digest::sha256(input)
}

pub(crate) fn lower_hex(bytes: &[u8]) -> String {
    invokrum_digest::lower_hex(bytes)
}

#[cfg(test)]
mod tests {
    use super::{digest, lower_hex};

    #[test]
    fn compatibility_wrapper_matches_published_sha256_vectors() {
        assert_eq!(
            lower_hex(&digest(b"")),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            lower_hex(&digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
