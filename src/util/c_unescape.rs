use std::borrow::Cow;

/// Reverses GLib's `g_strescape`, as `g_strcompress` does: C escapes and octal bytes, so a
/// localized prompt the helper escaped byte by byte decodes back to UTF-8.
pub fn c_unescape(escaped: &str) -> Cow<'_, str> {
    if !escaped.contains('\\') {
        return Cow::Borrowed(escaped);
    }
    let mut bytes = Vec::with_capacity(escaped.len());
    let mut input = escaped.bytes().peekable();
    while let Some(byte) = input.next() {
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        let Some(escape) = input.next() else { break };
        bytes.push(match escape {
            b'0'..=b'7' => {
                let mut value = escape - b'0';
                for _ in 0..2 {
                    match input.peek() {
                        Some(&digit @ b'0'..=b'7') => {
                            value = value.wrapping_mul(8).wrapping_add(digit - b'0');
                            input.next();
                        }
                        _ => break,
                    }
                }
                value
            }
            b'b' => 0x08,
            b'f' => 0x0c,
            b'n' => b'\n',
            b'r' => b'\r',
            b't' => b'\t',
            b'v' => 0x0b,
            literal => literal,
        });
    }
    Cow::Owned(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_borrowed() {
        assert!(matches!(
            c_unescape("Password: "),
            Cow::Borrowed("Password: ")
        ));
    }

    #[test]
    fn decodes_c_escapes() {
        assert_eq!(c_unescape(r#"a\tb\nc\\d\"e"#), "a\tb\nc\\d\"e");
    }

    #[test]
    fn decodes_octal_utf8_bytes() {
        assert_eq!(c_unescape(r"Passwort f\303\274r"), "Passwort für");
    }

    #[test]
    fn stops_at_a_trailing_backslash() {
        assert_eq!(c_unescape(r"end\"), "end");
    }
}
