#![feature(test)]

#[macro_use]
extern crate arrayref;

extern crate byteorder;
extern crate rand;
extern crate resize_slice;
extern crate time;

mod base62;

use std::io;
use std::ascii::AsciiExt;

use byteorder::{ByteOrder, BigEndian};
use time::{Timespec, Tm, Duration};
use rand::{Rng, Rand};

pub const KSUID_EPOCH: Timespec = Timespec {sec: 1_400_000_000, nsec: 0};

const LEN: usize = 20;
const EMPTY: [u8; LEN] = [0; LEN];
const BASE62_LEN: usize = 27;
const HEX_LEN: usize = 40;
const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";
const MAX_BASE62_KSUID: &[u8] = b"aWgEPTl1tmebfsQzFP4bxwgy80V";

/// A 20-byte UUID.
///
/// The first 4 bytes are a big-endian encoded, unsigned timestamp indicating when the UUID was
/// created. The timestamp is relative to a custom epoch which is defined to be 14e8 seconds after
/// the UNIX epoch.
///
/// The remaining 16 bytes is a random payload, much like UUIDv4.
#[derive(Clone, Copy, Ord, PartialOrd, Eq, PartialEq)]
pub struct Ksuid([u8; 20]);

impl Ksuid {
    /// Create a new `Ksuid` with the given timestamp and payload.
    pub fn new(timestamp: u32, payload: [u8; 16]) -> Self {
        let mut ret = Ksuid(EMPTY);
        ret.set_timestamp(timestamp);
        ret.set_payload(payload);
        ret
    }

    /// Create a new `Ksuid` with a current timestamp and the given payload.
    pub fn with_payload(payload: [u8; 16]) -> Self {
        // TODO: check for overflow in timestamp
        let elapsed = time::get_time() - KSUID_EPOCH;
        let ts = elapsed.num_seconds() as u32;
        Self::new(ts, payload)
    }

    /// Create a new `Ksuid` with a current timestamp and randomly generated payload.
    ///
    /// This function uses the thread local random number generator. This means that if you're
    /// calling `generate()` in a loop, caching the generator can increase performance. See the
    /// documentation of [`rand::random()`](https://doc.rust-lang.org/rand/rand/fn.random.html) for
    /// an example.
    pub fn generate() -> Self {
        rand::random()
    }

    /// Parse a `Ksuid` from a 27-byte, base62-encoded string.
    pub fn from_base62(s: &str) -> io::Result<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != BASE62_LEN || bytes > MAX_BASE62_KSUID {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid id"));
        }

        Self::from_base62_exact(array_ref![s.as_bytes(), 0, BASE62_LEN])
    }

    fn from_base62_exact(s: &[u8; BASE62_LEN]) -> io::Result<Self> {
        if s.as_ref() > MAX_BASE62_KSUID {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid id"));
        }

        let mut ret = Ksuid(EMPTY);
        let mut scratch = *s;
        base62::decode_raw(scratch.as_mut(), ret.0.as_mut())?;
        Ok(ret)
    }

    /// Parse a `Ksuid` from a valid hex-encoded string. All characters in `hex` must match
    /// `/[0-9A-Fa-f]/`.
    ///
    /// Once decoded, `hex` must be exactly 20 bytes long
    pub fn from_hex(hex: &str) -> io::Result<Self> {
        if hex.len() != HEX_LEN {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Hex string must be 40 bytes long"));
        }

        Self::from_hex_exact(array_ref![hex.as_bytes(), 0, HEX_LEN])
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid hex character in input"))
    }

    fn from_hex_exact(hex: &[u8; 40]) -> Result<Self, ()> {
        let mut ret = Ksuid(EMPTY);

        for (pair, place) in hex.chunks(2).zip(ret.0.iter_mut()) {
            let upper = HEX_DIGITS.iter().position(|d| pair[0].eq_ignore_ascii_case(d)).ok_or(())?;
            let lower = HEX_DIGITS.iter().position(|d| pair[1].eq_ignore_ascii_case(d)).ok_or(())?;
            *place = (upper * 16 + lower) as u8;
        }

        Ok(ret)
    }

    /// Parse a `Ksuid` from a raw slice.
    ///
    /// `raw` must be exactly 20 bytes long.
    pub fn from_bytes(raw: &[u8]) -> Result<Self, ()> {
        if raw.len() != LEN {
            return Err(())
        }

        Ok(Self::from_bytes_exact(array_ref![raw, 0, LEN]))
    }

    fn from_bytes_exact(hex: &[u8; 20]) -> Self {
        let mut ret = Ksuid(EMPTY);
        ret.0.copy_from_slice(hex.as_ref());
        ret
    }

    /// The base62-encoded version of this `Ksuid`.
    pub fn to_base62(&self) -> String {
        let mut scratch = self.0;
        let mut out = vec![0; 27];
        base62::encode_raw(scratch.as_mut(), out.as_mut());
        unsafe { String::from_utf8_unchecked(out) }
    }

    /// The hex-encoded version of this `Ksuid`.
    pub fn to_hex(&self) -> String {
        let mut ret = String::with_capacity(40);
        for b in self.as_bytes() {
            ret.push(HEX_DIGITS[(b / 16) as usize] as char);
            ret.push(HEX_DIGITS[(b % 16) as usize] as char);
        }
        ret
    }

    /// The binary representation of this `Ksuid`.
    ///
    /// This function does not allocate.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_ref()
    }

    /// The number of seconds after the custom KSUID epoch when this id was created.
    pub fn timestamp(&self) -> u32 {
        BigEndian::read_u32(self.0.as_ref())
    }

    /// Set the timestamp.
    pub fn set_timestamp(&mut self, timestamp: u32) {
        BigEndian::write_u32(self.0.as_mut(), timestamp);
    }

    /// The 16-byte random payload.
    pub fn payload(&self) -> [u8; 16] {
        *array_ref![&self.0, 4, 16]
    }

    /// Set the 16-byte payload.
    pub fn set_payload(&mut self, payload: [u8; 16]) {
        (&mut self.0[4..]).copy_from_slice(payload.as_ref());
    }
}

impl Ksuid {
    /// A `Timespec` containing the number of seconds after the UNIX epoch when this `Ksuid` was
    /// created.
    pub fn timespec(&self) -> Timespec {
        KSUID_EPOCH + Duration::seconds(self.timestamp() as i64)
    }

    /// The time at which this `Ksuid` was created, in the user's local time.
    pub fn time(&self) -> Tm {
        time::at(self.timespec())
    }

    /// The time at which this `Ksuid` was created, normalized to UTC±00:00.
    pub fn time_utc(&self) -> Tm {
        time::at_utc(self.timespec())
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
        let ksuid = Ksuid::from_bytes(&[255; 20]).unwrap();

        b.iter(|| {
            ksuid.to_base62()
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
