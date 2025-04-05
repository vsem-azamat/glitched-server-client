pub fn encode(bytes: &[u8]) -> String {
    const HEX_DIGITS: &[u8] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        result.push(HEX_DIGITS[(byte >> 4) as usize] as char);
        result.push(HEX_DIGITS[(byte & 0xF) as usize] as char);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        let input: [u8; 0] = [];
        assert_eq!(encode(&input), "");
    }

    #[test]
    fn test_single_byte() {
        assert_eq!(encode(&[0x00]), "00");
        assert_eq!(encode(&[0xFF]), "ff");
    }

    #[test]
    fn test_multiple_bytes() {
        assert_eq!(encode(&[0xDE, 0xAD, 0xBE, 0xEF]), "deadbeef");
        assert_eq!(
            encode(&[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0]),
            "123456789abcdef0"
        );
    }
}
