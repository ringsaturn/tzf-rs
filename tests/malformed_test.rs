//! Structural validation: a mutated file must fail closed with a clean error
//! (spec §8.1) — never a panic, and never a silently wrong open.

#[cfg(test)]
#[cfg(feature = "bundled")]
mod tests {
    use tzf_rs::{DefaultFinder, EmbeddedFinder, Error};

    fn lite() -> Vec<u8> {
        tzf_dist::load_lite_tzb().to_vec()
    }

    #[test]
    fn empty_and_truncated_files_are_rejected() {
        assert!(DefaultFinder::from_tzb(&[]).is_err());
        assert!(EmbeddedFinder::from_tzb(Vec::new()).is_err());
        let data = lite();
        for len in [1, 4, 63, 64, 1024, data.len() - 1] {
            assert!(
                DefaultFinder::from_tzb(&data[..len]).is_err(),
                "truncation to {len} bytes accepted"
            );
        }
    }

    #[test]
    fn bad_magic_is_rejected() {
        let mut data = lite();
        data[0] = b'X';
        assert!(matches!(
            DefaultFinder::from_tzb(&data),
            Err(Error::Malformed(_))
        ));
    }

    #[test]
    fn wrong_format_major_is_rejected() {
        let mut data = lite();
        data[4] = 9;
        assert!(DefaultFinder::from_tzb(&data).is_err());
    }

    #[test]
    fn unknown_profile_is_rejected() {
        let mut data = lite();
        data[48] = 7;
        assert!(DefaultFinder::from_tzb(&data).is_err());
    }

    #[test]
    fn corrupted_payload_fails_crc() {
        let mut data = lite();
        let mid = data.len() / 2;
        data[mid] ^= 0xff;
        assert!(matches!(
            DefaultFinder::from_tzb(&data),
            Err(Error::Malformed("CRC32"))
        ));
    }

    #[test]
    fn m_profile_files_are_rejected() {
        // tzf-rs consumes the .tzb (E) profile only; a memory image must be
        // refused with the dedicated Profile error. The profile byte is
        // checked before the CRC, so flipping it exercises that path.
        let mut data = lite();
        data[48] = 1; // PROFILE_M
        assert!(matches!(
            DefaultFinder::from_tzb(&data),
            Err(Error::Profile)
        ));
        assert!(matches!(
            EmbeddedFinder::from_tzb(data),
            Err(Error::Profile)
        ));
    }
}
