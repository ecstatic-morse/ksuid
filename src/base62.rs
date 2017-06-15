//! Utilities for Base62 encoding of data.
//!
//! The public interface of this module is unusual in two ways:
//!
//! The `input` buffer is clobbered during encoding/decoding. This saves an allocation during the
//! change of base routine, as we can resue the buffer to hold intermediate values during long
//! division.
//!
//! The `output` buffer must be preallocated by the caller. If the caller fails to reserve enough
//! space, the function will panic. This choice was made because binary and string encoded KSUIDs
//! have known lengths, allowing us to avoid dynamically allocating the output buffer. For general
//! purpose use, callers should use `conversion_len_bound()` to calculate the required output
//! buffer length.

use std::io;

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

/// Convert an ascii Base62 character to its binary representation.
/// Returns `-1` if the  character was not valid Base62 (`[0-9A-Za-z]`).
///
/// # Panics
///
/// Panics if `c` is not in the 7-bit ASCII range (`c > 127`)
fn b62_to_bin(c: u8) -> i8 {
    // Map lookup is faster than indexing CHAR_MAP.
    BYTE_MAP[usize::from(c)]
}

/// An upper-bound on the length of the result of a generic base conversion.
pub fn conversion_len_bound(len: usize, in_base: usize, out_base: usize) -> usize {
    let out = len as f64 * ((in_base as f64).ln() / (out_base as f64).ln());
    out as usize + 1
}

/// Change the base of a byte string representing a big-endian encoded arbitrary-size unsigned
/// integer.
///
/// The result of the change of base will be written to the out buffer in big-endian order (the
/// least significant byte at buf.last()).
///
/// `out` should be zeroed by the caller prior to invoking this function, as `change_base()` does
/// not always need the whole output buffer to encode the result. When the buffer is zeroed, the
/// untouched bytes become leading zeros of the resulting integer.
fn change_base(mut num: &mut [u8], out: &mut [u8], in_base: usize, out_base: usize) {
    debug_assert!(out.iter().all(|&b| b == 0));

    let mut k = out.len();

    // Use grade-school long division, storing the intermediate result back into `num` as we go.
    while num.len() > 0 {
        let mut rem = 0;
        let mut i = 0;

        for j in 0..num.len() {
            let acc = usize::from(num[j]) + in_base * rem;
            let div = acc / out_base;
            rem = acc % out_base;

            if i != 0 || div != 0 {
                num[i] = div as u8;
                i += 1;
            }
        }

        k -= 1;
        *out.get_mut(k).expect("Input buffer not large enough") = rem as u8;
        out[k] = rem as u8;
        num.resize_to(i);
    }
}

/// Base62-encode `input`, placing the result into `output`.
pub fn encode_raw(input: &mut [u8], output: &mut [u8]) {
    change_base(input, output, 256, 62);
    for b in output.iter_mut() {
        *b = CHAR_MAP[usize::from(*b)];
    }
}

/// Decode the Base62-encoded data in `input`, placing the result into `output`. If `input`
/// contains any characters which do not match `/[0-9A-Za-z]/`, an error will be returned.
pub fn decode_raw(input: &mut [u8], output: &mut [u8]) -> io::Result<()> {
    // Map each ASCII-encoded Base62 character to its binary value.
    for c in input.iter_mut() {
        if *c & 0x80 != 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Non-ASCII character in input"));
        }

        let b = b62_to_bin(*c);
        if b < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid base62 character in input"));
        }

        *c = b as u8;
    }

    change_base(input, output, 62, 256);
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate num;
    extern crate data_encoding;
    extern crate test;
    use super::*;

    fn big_int(bytes: &[u8]) -> num::BigUint {
        num::BigUint::from_bytes_be(bytes)
    }

    #[test]
    fn tables() {
        assert_eq!(b62_to_bin(b'0'), 0);
        assert_eq!(b62_to_bin(b'A'), 10);
        assert_eq!(b62_to_bin(b'a'), 36);
    }

    #[test]
    fn test_change_base() {
        let suite = vec![
            (256, 62, vec![255u8, 254, 253, 252]),
        ];

        for (in_base, out_base, input) in suite {
            println!("input: {}", big_int(input.as_ref()));
            let mut intermediate = vec![0; conversion_len_bound(input.len(), in_base, out_base)];
            let mut output = vec![0; input.len()];

            change_base(input.clone().as_mut_slice(), intermediate.as_mut(), in_base, out_base);
            println!("intermediate: {}", big_int(intermediate.as_ref()));

            change_base(intermediate.clone().as_mut_slice(), output.as_mut(), out_base, in_base);
            let first_nonzero = output.iter().position(|&b| b != 0).unwrap();
            assert_eq!(input, output.split_at(first_nonzero).1);
        }
    }

    // These benchmarks don't zero the out buffer between runs to better isolate the performance of
    // the function under test.

    #[bench]
    fn bench_change_base_to_62(b: &mut test::Bencher) {
        let mut out = vec![0; 27];
        b.iter(|| {
            test::black_box(&mut out);
            let mut bytes = [255; 20];
            change_base(bytes.as_mut(), out.as_mut(), 256, 62);
        })
    }

    #[bench]
    fn bench_change_base_from_62(b: &mut test::Bencher) {
        let mut out = vec![0; 20];
        let mut bytes = vec![0; 27];
        let mut max_id = [0xff; 20];
        change_base(max_id.as_mut(), bytes.as_mut(), 256, 62);

        // `bytes` now holds the maximum valid Base62 encoded ksuid.
        b.iter(|| {
            test::black_box(&mut out);
            let mut scratch = [0; 27];
            scratch.copy_from_slice(bytes.as_ref());
            change_base(scratch.as_mut(), out.as_mut(), 62, 256);
        })
    }
}
