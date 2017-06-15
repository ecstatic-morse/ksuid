[![Version](https://img.shields.io/crates/v/ksuid.svg)](https://crates.io/crates/ksuid)
[![Docs](https://docs.rs/ksuid/badge.svg)](https://docs.rs/ksuid)

Ksuid
=====

KSUID stands for K-Sortable Unique IDentifier, a globally unique identifier used by
[Segment](https://segment.com/blog/a-brief-history-of-the-uuid/).

KSUIDs incorporate a timestamp with 1-second resolution, allowing them to be (roughly) sorted
chronologically, as well as a 128-bit random payload in the style of UUIDv4. They can be
serialized using a Base62 encoding for compatibility with environments which only support
alphanumeric data. The lexicographic ordering of both the binary and string representations
preserves the chronological ordering of the embedded timestamp.

See the [canonical implementation](https://github.com/segmentio/ksuid) for more information.

The author of this package is not affiliated with Segment.
