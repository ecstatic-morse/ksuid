//! KSUID stands for K-Sortable Unique IDentifier, a globally unique identifier created by
//! [Segment](https://segment.com/blog/a-brief-history-of-the-uuid/).
//!
//! KSUIDs incorporate a timestamp with 1-second resolution which allows them to be (roughly)
//! sorted chronologically, as well as a 128-bit random payload in the style of UUIDv4 which
//! reduces the risk of collisions. They can be serialized using a Base62 encoding compatible with
//! environments which only support alphanumeric data. The lexicographic ordering of both the
//! binary and string representations preserves the chronological ordering of the embedded
//! timestamp.
//!
//! See the [canonical implementation](https://github.com/segmentio/ksuid) for more information.
//!
//! The author of this package is not affiliated with Segment.

#![feature(test)]

extern crate byteorder;
extern crate rand;
extern crate resize_slice;
extern crate time;

mod base62;

use std::io;
use std::ascii::AsciiExt;

use byteorder::{ByteOrder, BigEndian};
use time::{Timespec, Duration};
use rand::{Rng, Rand};

/// The KSUID epoch, 1.4 billion seconds after the UNIX epoch.
///
/// ```
/// # extern crate ksuid;
/// # extern crate time;
/// assert_eq!(ksuid::EPOCH, time::strptime("2014-5-13 16:53:20", "%Y-%m-%d %T").unwrap().to_timespec());
/// ```
pub const EPOCH: Timespec = Timespec {sec: 1_400_000_000, nsec: 0};

const LEN: usize = 20;
const EMPTY: [u8; LEN] = [0; LEN];
const BASE62_LEN: usize = 27;
const HEX_LEN: usize = 40;
const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";
const MAX_BASE62_KSUID: &[u8] = b"aWgEPTl1tmebfsQzFP4bxwgy80V";

/// Get the numeric value corresponding to the given ASCII hex digit.
fn hex_digit(c: u8) -> io::Result<u8> {
    HEX_DIGITS.iter()
        .position(|d| c.eq_ignore_ascii_case(d))
        .map(|idx| idx as u8)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid hex character in input"))
}

/// A K-Sortable Unique IDentifier.
///
/// The first 4 bytes are a big-endian encoded, unsigned timestamp indicating when the UUID was
/// created. The timestamp is relative to a custom epoch, exported from this crate as
/// [`EPOCH`](constant.EPOCH.html).
///
/// The remaining 16 bytes is the randomly generated payload.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct Ksuid([u8; LEN]);

impl Ksuid {
    /// Create a new identifier with the given timestamp and payload.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ksuid::Ksuid::new(1000, [0; 16]);
    /// assert_eq!(id.timestamp(), 1000);
    /// assert_eq!(id.payload(), [0; 16]);
    /// ```
    pub fn new(timestamp: u32, payload: [u8; 16]) -> Self {
        let mut ret = Ksuid(EMPTY);
        ret.set_timestamp(timestamp);
        ret.set_payload(payload);
        ret
    }

    /// Create a new identifier with a current timestamp and the given payload.
    pub fn with_payload(payload: [u8; 16]) -> Self {
        // TODO: check for overflow in timestamp
        let elapsed = time::get_time() - EPOCH;
        let ts = elapsed.num_seconds() as u32;
        Self::new(ts, payload)
    }

    /// Create a new identifier with a current timestamp and randomly generated payload.
    ///
    /// This function uses the thread local random number generator. this means that if you're
    /// calling `generate()` in a loop, caching the generator can increase performance. See the
    /// documentation of [`rand::random()`](https://doc.rust-lang.org/rand/rand/fn.random.html) for
    /// an example.
    pub fn generate() -> Self {
        rand::random()
    }

    /// Parse an identifier from its Base62-encoded string representation.
    ///
    /// `s` must be exactly 27 characters long.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ksuid::Ksuid::from_base62("0o5Fs0EELR0fUjHjbCnEtdUwQe3").unwrap();
    /// assert_eq!(id.timestamp(), 94985761);
    /// ```
    pub fn from_base62(s: &str) -> io::Result<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != BASE62_LEN || bytes > MAX_BASE62_KSUID {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid id"));
        }

        let mut ret = Ksuid(EMPTY);
        let mut scratch = [0; BASE62_LEN];
        scratch.clone_from_slice(bytes);
        base62::decode_raw(scratch.as_mut(), ret.0.as_mut())?;
        Ok(ret)
    }

    /// Parse an identifer from a string of hex characters (`/[0-9A-Fa-f]/`).
    ///
    /// `hex` must be exactly 40 characters long.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ksuid::Ksuid::from_hex("05a95e21D7B6Fe8CD7Cff211704d8E7B9421210B").unwrap();
    /// assert_eq!(id.timestamp(), 94985761);
    /// ```
    pub fn from_hex(hex: &str) -> io::Result<Self> {
        if hex.len() != HEX_LEN {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Hex string must be 40 bytes long"));
        }

        let mut ret = Ksuid(EMPTY);
        for (pair, place) in hex.as_bytes().chunks(2).zip(ret.0.iter_mut()) {
            let upper = hex_digit(pair[0])?;
            let lower = hex_digit(pair[1])?;
            *place = (upper * 16 + lower) as u8;
        }

        Ok(ret)
    }

    /// Parse an identifier from its binary representation.
    ///
    /// `raw` must be exactly 20 bytes long.
    pub fn from_bytes(raw: &[u8]) -> io::Result<Self> {
        if raw.len() != LEN {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Ksuids are 20 bytes long"));
        }

        let mut ret = Ksuid(EMPTY);
        ret.0.copy_from_slice(raw);
        Ok(ret)
    }

    /// The Base62-encoded version of this identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// let id = ksuid::Ksuid::new(::std::u32::MAX, [255; 16]);
    /// assert_eq!(id.to_base62(), "aWgEPTl1tmebfsQzFP4bxwgy80V");
    /// ```
    pub fn to_base62(&self) -> String {
        let mut scratch = self.0;
        let mut out = vec![0; BASE62_LEN];
        base62::encode_raw(scratch.as_mut(), out.as_mut());

        // This is valid because base 62 encoded data contains only ASCII alphanumeric characters.
        unsafe { String::from_utf8_unchecked(out) }
    }

    /// The hex-encoded version of this identifier.
    pub fn to_hex(&self) -> String {
        let mut ret = Vec::with_capacity(HEX_LEN);
        for b in self.as_bytes() {
            ret.push(HEX_DIGITS[(b / 16) as usize]);
            ret.push(HEX_DIGITS[(b % 16) as usize]);
        }

        // This is valid because we push only ASCII characters from `HEX_DIGITS` into `ret`.
        unsafe { String::from_utf8_unchecked(ret) }
    }

    /// The 20-byte binary representation of this identifier.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// The 32-bit timestamp of this identifier.
    ///
    /// Most consumers should use [`Ksuid::time()`](struct.Ksuid.html#method.time) to extract
    /// the creation date of an identifer.
    pub fn timestamp(&self) -> u32 {
        BigEndian::read_u32(self.0.as_ref())
    }

    /// Set the 32-bit timestamp.
    pub fn set_timestamp(&mut self, timestamp: u32) {
        BigEndian::write_u32(self.0.as_mut(), timestamp);
    }

    /// The number of seconds after the UNIX epoch when this identifier was created.
    pub fn time(&self) -> Timespec {
        EPOCH + Duration::seconds(self.timestamp().into())
    }

    /// Set the timestamp of the identifier to the given time.
    pub fn set_time(&mut self, time: Timespec) {
        let dur = time - EPOCH;
        self.set_timestamp(dur.num_seconds() as u32);
    }

    /// The 16-byte random payload.
    pub fn payload(&self) -> &[u8] {
        &self.0[4..]
    }

    /// Set the 16-byte payload.
    pub fn set_payload(&mut self, payload: [u8; 16]) {
        (&mut self.0[4..]).copy_from_slice(payload.as_ref());
    }
}

impl Rand for Ksuid {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        Self::with_payload(rng.gen())
    }
}

#[cfg(test)]
mod tests {
    extern crate test;
    use super::*;

    #[bench]
    fn bench_from_base62(b: &mut test::Bencher) {
        let encoded = ::std::str::from_utf8(MAX_BASE62_KSUID).unwrap();

        b.iter(|| {
            Ksuid::from_base62(encoded)
        })
    }

    #[bench]
    fn bench_to_base62(b: &mut test::Bencher) {
        let ksuid = Ksuid::from_bytes(&[255; LEN]).unwrap();

        b.iter(|| {
            ksuid.to_base62()
        })
    }

    #[bench]
    fn bench_to_hex(b: &mut test::Bencher) {
        let ksuid = Ksuid::from_bytes(&[255; LEN]).unwrap();

        b.iter(|| {
            ksuid.to_hex()
        })
    }

    #[bench]
    fn bench_from_hex(b: &mut test::Bencher) {
        b.iter(|| {
            Ksuid::from_hex("ffffffffffffffffffffffffffffffffffffffff")
        })
    }

    #[bench]
    fn bench_gen(b: &mut test::Bencher) {
        b.iter(|| {
            Ksuid::generate()
        })
    }

    #[bench]
    fn bench_gen_lock_rng(b: &mut test::Bencher) {
        let mut rng = rand::thread_rng();
        b.iter(|| {
            rng.gen::<Ksuid>()
        })
    }
}
