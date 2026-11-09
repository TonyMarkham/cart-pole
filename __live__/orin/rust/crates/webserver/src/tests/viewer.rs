//! The viewer's base64 decoder, against RFC 4648's own test vectors
//! (section 10): "Zm9v" is "foo", "Zm9vYg==" is "foob", "Zm9vYmE=" is
//! "fooba", and "Zm9vYmFy" is "foobar".

use crate::viewer::base64::decode;

#[test]
fn given_rfc_4648s_test_vectors_when_decoded_then_each_gives_its_text() {
    assert_eq!(decode(""), Some(Vec::new()));
    assert_eq!(decode("Zg==").as_deref(), Some(&b"f"[..]));
    assert_eq!(decode("Zm8=").as_deref(), Some(&b"fo"[..]));
    assert_eq!(decode("Zm9v").as_deref(), Some(&b"foo"[..]));
    assert_eq!(decode("Zm9vYg==").as_deref(), Some(&b"foob"[..]));
    assert_eq!(decode("Zm9vYmE=").as_deref(), Some(&b"fooba"[..]));
    assert_eq!(decode("Zm9vYmFy").as_deref(), Some(&b"foobar"[..]));
}

/// A JPEG starts with the bytes FF D8 FF, which base64 writes as "/9j/".
#[test]
fn given_a_jpegs_start_when_decoded_then_its_marker_comes_out() {
    assert_eq!(decode("/9j/").as_deref(), Some(&[0xFF, 0xD8, 0xFF][..]));
}

#[test]
fn given_a_character_outside_the_alphabet_when_decoded_then_it_is_refused() {
    assert_eq!(decode("Zm9v!"), None);
}
