# Third-Party Notices & Donor Provenance (`stdlib/`)

The foundational Pilot Standard Library (`stdlib/`) is authored in 100% pure, zero-external-dependency idiomatic Pilot. Algorithmic and container scaffolds were bootstrapped using Pilot's `M20` clean-room transpiler (`pilot --from <python|rust|c> --strict-pilot --viable-license-only`) exclusively from permissive (`MIT`, `Apache-2.0`, `PSF-2.0`, `CC0-1.0`) sources and subsequently curated for Pilot's `T?` niche null-safety, `T ! E` values-first error handling, `SliceBuffer<T>` zero-copy views, and `comptime` evaluation.

---

## 1. Python Standard Library Algorithmic Scaffolds (`PSF-2.0`)
- **Modules**: `std.unittest` (`TestCase`, `TestSuite`, `TestResult`, `TextTestRunner`, `assertEqual`, `assertTrue`, `assertFalse`, `assertIn`, `assertAlmostEqual`), `std.logging` (`Logger`, `Handler`, `StreamHandler`, `FileHandler`, `Formatter`, `Filter`, `LogRecord`, `getLogger`, `basicConfig`, `DEBUG/INFO/WARNING/ERROR/CRITICAL`), `std.timeit` (`Timer`, `default_timer`, `timeit`, `repeat`, `autorange`), `std.socket` (`Socket`, `AF_INET/6/UNIX`, `SOCK_STREAM/DGRAM/RAW`, `create_connection`, `inet_aton`, `inet_ntoa`), `std.urllib` (`urlparse`, `urlunparse`, `urlsplit`, `urlunsplit`, `urljoin`, `quote`, `quote_plus`, `unquote`, `unquote_plus`, `urlencode`, `parse_qs`, `parse_qsl`, `urlopen`, `urlretrieve`, `Request`, `HTTPResponse`, `URLError`, `HTTPError`), `std.http` (`HTTPStatus` constants/codes/phrases, `HTTPConnection`, `HTTPSConnection`, `HTTPResponse`, `HTTPServer`, `BaseHTTPRequestHandler`, `SimpleCookie`), `std.request` (`Session`, `Response`, `request`, `get_req`, `post`, `put`, `patch`, `delete`, `head`, `options`, cookie jar management, `raise_for_status`), `std.threading` (`Thread`, `Lock`, `RLock`, `Semaphore`, `BoundedSemaphore`, `Event`, `Barrier`, `ThreadLocal`), `std.multiprocessing` (`Process`, `Queue`, `Pipe`, `Pool`, `Value`, `Array`, `cpu_count`), `std.asyncio` (`EventLoop`, `Future`, `Task`, `run`, `gather`, `sleep`, `wait_for`, `AsyncLock`, `AsyncEvent`, `AsyncQueue`), `std.secrets` (`token_bytes`, `token_hex`, `token_urlsafe`, `randbelow`, `randbits`, `compare_digest`), `std.decimal` (`Decimal`, `Context`, `ROUND_*`), `std.math` (`fabs`, `copysign`, `trunc`, `fmod`, `hypot`, `dist`, `comb`, `perm`, `isclose`, `prod`, `degrees`, `radians`, Welford running variance/mean), `std.random` (`seed`, `random`, `uniform`, `randint`, `randrange`, `choice`, `choices`, `shuffle`, `sample`, `gauss`, `normalvariate`, `triangular`), `std.json` (`loads`, `dumps`, `load`, `dump`, `JSONDecoder`, `JSONEncoder`, `JSONDecodeError`), `std.csv` (`reader`, `writer`, `DictReader`, `DictWriter`, `excel`, `unix`, `QUOTE_MINIMAL/ALL/NONNUMERIC/NONE`), `std.pickle` (`dumps`, `loads`, `dump`, `load`, `Pickler`, `Unpickler`), `std.tomllib` (`loads`, `load`, `dumps`, `dump`, `TOMLDecodeError`), `std.collections` (`Counter`, `defaultdict`, `OrderedDict`, `ChainMap`, `deque`, `namedtuple`, `bisect_left`, `bisect_right`, binary min-heap sift routines), `std.itertools` (`accumulate`, `batched`, `chain`, `compress`, `dropwhile`, `filterfalse`, `groupby`, `islice`, `pairwise`, `starmap`, `takewhile`, `tee`, `product`, `permutations`, `combinations`, `combinations_with_replacement`), `std.functools` (`partial`, `reduce`, `lru_cache`, `cache`, `singledispatch`, `cmp_to_key`), `std.path`, `std.fs`, `std.pathlib` (object-oriented `Path` manipulation), `std.shutil` (high-level file/tree operations, `which`, `disk_usage`), `std.os` (multi-os platform detection, `environ`, `getcwd`), `std.sys` (`argv`, `platform`, `arch`, `exit`), `std.encoding` (RFC-4648 `hex`/`base64`/`base64url` and RFC-3986 `url`/query percent-encoding), `std.formats` (RFC-8259 `json`, TOML v1.0 `toml` derived from `tomllib`/`tomli` MIT/PSF-2.0, RFC-4180 `csv`, and `.env` variable expansion).
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

---

## 4. Scientific Python Ecosystem Scaffolds (`BSD-3-Clause` / `PSF-2.0`)
- **Modules**:
  - `std.numpy`: `ndarray`, `array`, `zeros`, `ones`, `arange`, `linspace`, `eye`, `matmul`, `dot`, elementwise tensor vectorization, shape manipulation, statistical reducers (`mean`, `variance`, `std`, `sum`, `min`, `max`, `argmin`, `argmax`).
    - **SPDX-License-Identifier**: `BSD-3-Clause`
    - **Copyright**: Copyright (c) 2005–2026, NumPy Developers.
  - `std.pandas`: `Series`, `DataFrame`, labeled indexing, column projection, slicing, aggregation (`describe`), CSV serialization (`read_csv`, `to_csv`), dictionary record transformation (`from_dict`, `from_records`).
    - **SPDX-License-Identifier**: `BSD-3-Clause`
    - **Copyright**: Copyright (c) 2008–2026, AQR Capital Management, LLC, Lambda Foundry, Inc. and PyData Development Team.
  - `std.matplotlib`: `pyplot`, `Figure`, cross-platform vector SVG and terminal ASCII canvas rendering, line plots, scatter plots, bar charts, histograms, axis labels, titles, legends, grids.
    - **SPDX-License-Identifier**: `PSF-2.0 AND BSD-3-Clause`
    - **Copyright**: Copyright (c) 2002–2026 John D. Hunter, Michael Droettboom, and the Matplotlib development team.
  - `std.scikit`: `LinearRegression`, `LogisticRegression`, `KNeighborsClassifier`, `KMeans`, `StandardScaler`, `MinMaxScaler`, `LabelEncoder`, `train_test_split`, `KFold`, `mean_squared_error`, `mean_absolute_error`, `r2_score`, `accuracy_score`, `confusion_matrix`, `precision_score`, `recall_score`, `f1_score`.
    - **SPDX-License-Identifier**: `BSD-3-Clause`
    - **Copyright**: Copyright (c) 2007–2026 The scikit-learn developers.
