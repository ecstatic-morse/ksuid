use resize_slice::ResizeSlice;

const CHAR_MAP: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

const BYTE_MAP: &[i8] = &[
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
    -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1, -1,
     0,  1,  2,  3,  4,  5,  6,  7,  8,  9, -1, -1, -1, -1, -1, -1,
    -1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
    25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, -1, -1, -1, -1, -1,
    -1, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, -1, -1, -1, -1, -1,
];

/// Convert an ascii base-62 character to its binary representation.
/// Returns `-1` if the  character was not valid base-62 (`[0-9A-Za-z]`).
///
/// # Panics
///
/// Panics if `c` is not in the ASCII range (`c > 127`)
fn b62_to_bin(c: u8) -> i8 {
    // TODO: benchmark vs position() in CHAR_MAP
    BYTE_MAP[c as usize]
}

/// An upper-bound on the length of the result of a generic base conversion.
fn upper_bound(len: usize, in_base: usize, out_base: usize) -> usize {
    let out = len as f64 * ((in_base as f64).ln() / (out_base as f64).ln());
    out as usize + 1
}

/// An upper bound on the length of the base62-encoded version of a byte string.
fn encoded_upper_bound(len: usize) -> usize {
    // log(256) / log(62) = 1.343590...
    1 + (len * 13446) / 10000
}

/// An upper bound on the length of the decoded version of a base62-encoded byte string.
fn decoded_upper_bound(len: usize) -> usize {
    // log(62) / log(256) = 0.74427453...
    1 + (len * 7443) / 10000
}

/// Change the base of a byte string.
pub fn change_base(mut num: &mut [u8], in_base: usize, out_base: usize) -> Vec<u8> {
    let out_len = match (in_base, out_base) {
        (256, 62) => encoded_upper_bound(num.len()),
        (62, 256) => decoded_upper_bound(num.len()),
        _         => upper_bound(num.len(), in_base, out_base),
    };

    let mut out = Vec::with_capacity(out_len);

    while num.len() > 0 {
        let mut rem = 0;
        let mut i = 0;

        for j in 0..num.len() {
            let acc = num[j] as usize + in_base * rem;
            let div = acc / out_base;
            rem = acc % out_base;

            if i != 0 || div != 0 {
                num[i] = div as u8;
                i += 1;
            }
        }

        out.push(rem as u8);
        num.resize_to(i);
    }

    out.reverse();
    out
}

pub fn encode_raw(raw: &mut [u8]) -> String {
    let mut encoded = change_base(raw, 256, 62);
    for b in encoded.iter_mut() {
        *b = CHAR_MAP[*b as usize];
    }

    unsafe { String::from_utf8_unchecked(encoded) }
}

pub fn decode_raw(encoded: &mut [u8]) -> Result<Vec<u8>, char> {
    // Map each ASCII-encoded base-62 character to its binary value.
    for c in encoded.iter_mut() {
        if *c & 0x80 != 0 {
            return Err(*c as char);
        }

        let b = b62_to_bin(*c);
        if b == -1 {
            return Err(*c as char)
        }

        *c = b as u8;
    }

    let decoded = change_base(encoded, 62, 256);
    Ok(decoded)
}

#[cfg(test)]
mod tests {
    extern crate data_encoding;
    extern crate test;
    use super::*;

    fn big_int(bytes: &[u8]) -> u64 {
        let mut ret = 0;
        let mut mul = 1;
        for &b in bytes.iter().rev() {
            ret += mul * b as u64;
            mul *= 256
        }

        ret
    }

    #[test]
    fn tables() {
        assert_eq!(b62_to_bin(b'0'), 0);
        assert_eq!(b62_to_bin(b'A'), 10);
        assert_eq!(b62_to_bin(b'a'), 36);
    }

    #[test]
    fn test_bounds() {
        for i in 0..10000 {
            let (approx, exact) = (decoded_upper_bound(i), upper_bound(i, 62, 256));
            assert!(approx >= exact, "dec: {} < {} [i={}]", approx, exact, i);

            let (approx, exact) = (encoded_upper_bound(i), upper_bound(i, 256, 62));
            assert!(approx >= exact, "enc: {} < {} [i={}]", approx, exact, i);
        }
    }

    #[test]
    fn test_change_base() {
        let suite = vec![
            (256, 62, vec![255u8, 254, 253, 252]),
        ];

        for (in_base, out_base, input) in suite {
            println!("input: {}", big_int(input.as_ref()));

            let intermediate = change_base(input.clone().as_mut_slice(), in_base, out_base);
            println!("intermediate: {}", big_int(intermediate.as_ref()));

            let output = change_base(intermediate.clone().as_mut_slice(), out_base, in_base);
            assert_eq!(input, output);
        }
    }

    #[bench]
    fn bench_change_base(b: &mut test::Bencher) {
        b.iter(|| {
            let mut bytes = [12, 104, 48, 1, 245, 234, 245, 14, 194];
            change_base(&mut bytes[..], 256, 62)
        })
    }

    #[bench]
    fn bench_encode(b: &mut test::Bencher) {
        let hex = data_encoding::hex::decode(b"05A95E21D7B6FE8CD7CFF211704D8E7B9421210B").unwrap();
        b.iter(|| {
            encode_raw(&mut hex.clone()[..])
        })
    }

    #[bench]
    fn bench_decode(b: &mut test::Bencher) {
        let encoded = *array_ref!(b"0o5Fs0EELR0fUjHjbCnEtdUwQe3", 0, 27);

        b.iter(|| {
            decode_raw(&mut encoded.clone()[..])
        })
    }
}
