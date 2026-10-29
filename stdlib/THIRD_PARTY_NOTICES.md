# Third-Party Notices & Donor Provenance (`stdlib/`)

The foundational Pilot Standard Library (`stdlib/`) is authored in 100% pure, zero-external-dependency idiomatic Pilot. Algorithmic and container scaffolds were bootstrapped using Pilot's `M20` clean-room transpiler (`pilot --from <python|rust|c> --strict-pilot --viable-license-only`) exclusively from permissive (`MIT`, `Apache-2.0`, `PSF-2.0`, `CC0-1.0`) sources and subsequently curated for Pilot's `T?` niche null-safety, `T ! E` values-first error handling, `SliceBuffer<T>` zero-copy views, and `comptime` evaluation.

---

## 1. Python Standard Library Algorithmic Scaffolds (`PSF-2.0`)
- **Modules**: `std.collections` (`bisect_left`, `bisect_right`, binary min-heap sift routines), `std.math` (Welford running variance/mean), `std.path` & `std.fs` (POSIX/cross-platform lexical segment normalization and `fnmatch` wildcard matching), `std.encoding` (RFC-4648 `hex`/`base64`/`base64url` and RFC-3986 `url`/query percent-encoding), `std.formats` (RFC-8259 `json`, TOML v1.0 `toml` derived from `tomllib`/`tomli` MIT/PSF-2.0, RFC-4180 `csv`, and `.env` variable expansion).
- **SPDX-License-Identifier**: `PSF-2.0`
- **Copyright**: Copyright (c) 2001–2026 Python Software Foundation; All Rights Reserved.
- **Notice**: Adapted into idiomatic Pilot with half-open/inclusive range semantics, zero-copy `SliceBuffer<u8>` byte cursors, and `T?` / `T ! IoError` / `T ! DecodeError` / `T ! ParseError` return contracts.

---

## 2. Rust Core / Alloc Container Scaffolds (`MIT OR Apache-2.0`)
- **Modules**: `std.collections` (`Deque`, `RingBuffer`, `BitSet` SWAR 64-bit word operations), `std.option`, `std.result`.
- **SPDX-License-Identifier**: `MIT OR Apache-2.0`
- **Copyright**: Copyright (c) The Rust Project Developers.

---

## 3. SplitMix64 & PCG-XSH-RR Deterministic PRNGs (`CC0-1.0` / `MIT` / `Apache-2.0`)
- **Modules**: `std.random` (`SplitMix64`, `PcgRng`).
- **SplitMix64**: Written in 2015 by Sebastiano Vigna (`CC0-1.0` / Public Domain).
- **PCG Random Number Generation**: Copyright (c) 2014 Melissa E. O'Neill (`MIT OR Apache-2.0`).
