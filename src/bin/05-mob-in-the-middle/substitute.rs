use std::borrow::Cow;

const MIN_LEN: usize = 26;
const MAX_LEN: usize = 35;

pub fn substitute_boguscoin_address<'a>(line: &'a str, replacement: &str) -> Cow<'a, str> {
    let bytes = line.as_bytes();
    let mut out: Option<String> = None;

    let mut scan = 0usize;
    let mut last_copied = 0usize;

    while scan < bytes.len() {
        if let Some(end) = match_address(bytes, scan) {
            let buf = out.get_or_insert_with(|| String::with_capacity(line.len()));

            buf.push_str(&line[last_copied..scan]);
            buf.push_str(replacement);

            last_copied = end;
            scan = end;
        } else {
            scan += 1;
        }
    }

    match out {
        None => Cow::Borrowed(line),
        Some(mut buf) => {
            buf.push_str(&line[last_copied..]);
            Cow::Owned(buf)
        }
    }
}

fn match_address(bytes: &[u8], start: usize) -> Option<usize> {
    if start > 0 && bytes[start - 1] != b' ' {
        return None;
    }

    if bytes.get(start)? != &b'7' {
        return None;
    }

    let mut end = start + 1;

    while end < bytes.len() && end - start < MAX_LEN && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }

    let len = end - start;

    if (MIN_LEN..=MAX_LEN).contains(&len) && (end == bytes.len() || bytes[end] == b' ') {
        Some(end)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_single_address() {
        let input = "Please send the payment of 750 Boguscoins to 7PM5y5HTsk8cwYxc2C6jflP2SkvNMO";
        let address = "7replacementaddress123456789012345";
        let result = substitute_boguscoin_address(input, address);
        assert!(matches!(result, Cow::Owned(_)));
        assert_eq!(
            result,
            "Please send the payment of 750 Boguscoins to 7replacementaddress123456789012345"
        );
    }

    #[test]
    fn word_boundaries() {
        let input = "This is a product ID, not a Boguscoin: 7NnV1r2d7gE0wGU2QL9tpB8nCx-IOmwKkOdHwlKPjYcXGr9tV0MEzofUrHFA-1234";
        let address = "7replacementaddress123456789012345";
        let result = substitute_boguscoin_address(input, address);
        assert_eq!(result, input);
    }

    #[test]
    fn test_substitute_multiple_addresses() {
        let input = "7abc1234567890123456789012345 and 7def9876543210987654321098765";
        let address = "7replacementaddress123456789012345";
        let result = substitute_boguscoin_address(input, address);
        assert_eq!(
            result,
            "7replacementaddress123456789012345 and 7replacementaddress123456789012345"
        );
    }

    #[test]
    fn test_substitute_multiple_addresses_only_space_separator() {
        let input = "7abc1234567890123456789012345 7def9876543210987654321098765";
        let address = "7replacementaddress123456789012345";
        let result = substitute_boguscoin_address(input, address);
        assert_eq!(
            result,
            "7replacementaddress123456789012345 7replacementaddress123456789012345"
        );
    }

    #[test]
    fn test_no_address_in_line() {
        let input = "No address here!";
        let address = "7replacementaddress123456789012345";
        let result = substitute_boguscoin_address(input, address);
        assert_eq!(result, input);
    }
}
