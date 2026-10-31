## 2023-11-01 - Parallel Parsing Optimization

**Learning:** Leveraging multi-threaded parsing (`parse_parallel`) instead of single-threaded `parse` in CLI applications drastically improves performance for large corpora without changing program behavior. In `pilot`, this increased parse throughput from ~34MB/s to over ~50MB/s.
**Action:** Always search for multi-threaded/parallel alternatives to heavily utilized synchronous functions (like parsers or data loaders) in performance-critical code paths.
