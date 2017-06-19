[![Version](https://img.shields.io/crates/v/ksuid.svg)](https://crates.io/crates/ksuid)
[![Docs](https://docs.rs/ksuid/badge.svg)](https://docs.rs/ksuid)

Ksuid
=====

KSUID stands for K-Sortable Unique IDentifier, a globally unique
identifier used by
[Segment](https://segment.com/blog/a-brief-history-of-the-uuid/).

KSUIDs incorporate a timestamp with 1-second resolution, allowing them
to be (roughly) sorted chronologically, as well as a 128-bit random
payload in the style of UUIDv4. They can be serialized using a Base62
encoding for compatibility with environments which only support
alphanumeric data. The lexicographic ordering of both the binary and
string representations preserves the chronological ordering of the
embedded timestamp.

See the [canonical implementation](https://github.com/segmentio/ksuid)
for more information.

The author of this package is not affiliated with Segment.

This repository contains two separate crates, a library (`ksuid`) for
generating, parsing and serializing KSUIDs and a simple CLI
(`ksuid-cli`) which exposes a subset of this functionality for
interactive use.

Benchmarks
==========

The library includes some benchmarks to compare its performance against
the canonical implementation. However, the benchmarks use rust's
unstable `test` crate, so they are hidden behind a feature flag. Execute
`cargo bench --features bench` with a nightly version of the compiler to
run the benchmarks.

