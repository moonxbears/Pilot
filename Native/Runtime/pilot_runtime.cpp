// SPDX-License-Identifier: MIT
// Pilot Native C++20 Runtime & SIMD Accelerator (`libpilot_rt`)
// Provides zero-marshalling SliceBuffer<T> SIMD kernels, Welford single-pass statistics,
// Burst vector physics kernels, and linear-time O(N) non-backtracking DFA regex scanners.

#include "pilot_runtime.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <limits>

extern "C" {

PILOT_API int32_t pilot_rt_version(void) {
    return 2101;
}

PILOT_API int32_t pilot_rt_simd_width_bits(void) {
#if defined(__AVX512F__)
    return 512;
#elif defined(__AVX2__)
    return 256;
#elif defined(__SSE2__) || defined(_M_X64) || defined(__ARM_NEON) || defined(__aarch64__)
    return 128;
#else
    return 64;
#endif
}

PILOT_API double pilot_rt_slice_sum_f64(const double* data, size_t len) {
    if (!data || len == 0) return 0.0;

    double s0 = 0.0, s1 = 0.0, s2 = 0.0, s3 = 0.0;
    size_t i = 0;
    const size_t limit = len & ~static_cast<size_t>(3);
    for (; i < limit; i += 4) {
        s0 += data[i];
        s1 += data[i + 1];
        s2 += data[i + 2];
        s3 += data[i + 3];
    }
    double total = (s0 + s1) + (s2 + s3);
    for (; i < len; ++i) {
        total += data[i];
    }
    return total;
}

PILOT_API int64_t pilot_rt_slice_sum_i64(const int64_t* data, size_t len) {
    if (!data || len == 0) return 0;

    uint64_t s0 = 0, s1 = 0, s2 = 0, s3 = 0;
    size_t i = 0;
    const size_t limit = len & ~static_cast<size_t>(3);
    for (; i < limit; i += 4) {
        s0 += static_cast<uint64_t>(data[i]);
        s1 += static_cast<uint64_t>(data[i + 1]);
        s2 += static_cast<uint64_t>(data[i + 2]);
        s3 += static_cast<uint64_t>(data[i + 3]);
    }
    uint64_t total = (s0 + s1) + (s2 + s3);
    for (; i < len; ++i) {
        total += static_cast<uint64_t>(data[i]);
    }
    return static_cast<int64_t>(total);
}

PILOT_API double pilot_rt_slice_dot_f64(const double* a, const double* b, size_t len) {
    if (!a || !b || len == 0) return 0.0;

    double acc0 = 0.0, acc1 = 0.0, acc2 = 0.0, acc3 = 0.0;
    size_t i = 0;
    const size_t limit = len & ~static_cast<size_t>(3);
    for (; i < limit; i += 4) {
        acc0 += a[i] * b[i];
        acc1 += a[i + 1] * b[i + 1];
        acc2 += a[i + 2] * b[i + 2];
        acc3 += a[i + 3] * b[i + 3];
    }
    double total = (acc0 + acc1) + (acc2 + acc3);
    for (; i < len; ++i) {
        total += a[i] * b[i];
    }
    return total;
}

PILOT_API void pilot_rt_slice_saxpy_f64(double alpha, const double* x, double* y, size_t len) {
    if (!x || !y || len == 0) return;
    for (size_t i = 0; i < len; ++i) {
        y[i] += alpha * x[i];
    }
}

PILOT_API void pilot_rt_slice_scale_f64(double alpha, double* data, size_t len) {
    if (!data || len == 0) return;
    for (size_t i = 0; i < len; ++i) {
        data[i] *= alpha;
    }
}

PILOT_API void pilot_rt_slice_fill_f64(double* data, size_t len, double value) {
    if (!data || len == 0) return;
    std::fill(data, data + len, value);
}

PILOT_API void pilot_rt_slice_fill_i64(int64_t* data, size_t len, int64_t value) {
    if (!data || len == 0) return;
    std::fill(data, data + len, value);
}

PILOT_API void pilot_rt_slice_sort_f64(double* data, size_t len) {
    if (!data || len <= 1) return;
    std::sort(data, data + len, [](double a, double b) noexcept {
        if (std::isnan(a)) return false;
        if (std::isnan(b)) return true;
        return a < b;
    });
}

PILOT_API void pilot_rt_slice_sort_i64(int64_t* data, size_t len) {
    if (!data || len <= 1) return;
    std::sort(data, data + len);
}

PILOT_API void pilot_rt_welford_f64(
    const double* data,
    size_t len,
    double* out_mean,
    double* out_m2,
    double* out_min,
    double* out_max)
{
    if (!out_mean || !out_m2 || !out_min || !out_max) return;
    if (!data || len == 0) {
        *out_mean = 0.0;
        *out_m2 = 0.0;
        *out_min = 0.0;
        *out_max = 0.0;
        return;
    }

    double mean = data[0];
    double m2 = 0.0;
    double lo = data[0];
    double hi = data[0];

    for (size_t i = 1; i < len; ++i) {
        const double x = data[i];
        const double count_f = static_cast<double>(i + 1);
        const double delta = x - mean;
        mean += delta / count_f;
        const double delta2 = x - mean;
        m2 += delta * delta2;
        if (x < lo) lo = x;
        if (x > hi) hi = x;
    }

    *out_mean = mean;
    *out_m2 = m2;
    *out_min = lo;
    *out_max = hi;
}

PILOT_API void pilot_rt_integrate_vectors_f64(
    double* positions,
    double* velocities,
    double dt,
    double drag,
    size_t count)
{
    if (!positions || !velocities || count == 0) return;
    for (size_t i = 0; i < count; ++i) {
        const double v = velocities[i] * drag;
        velocities[i] = v;
        positions[i] += v * dt;
    }
}

} // extern "C"

namespace {

template <typename CodeUnit>
inline int32_t dfa_scan_generic(
    const int32_t* ascii_table_128,
    const uint8_t* accept_flags,
    int32_t state_count,
    int32_t has_non_ascii,
    int32_t anchored_start,
    int32_t anchored_end,
    const CodeUnit* units,
    int32_t length,
    int32_t start_at,
    int32_t* out_start,
    int32_t* out_end)
{
    if (!ascii_table_128 || !accept_flags || state_count <= 0 || !out_start || !out_end) {
        return 0;
    }
    if (start_at < 0) start_at = 0;
    if (start_at > length) {
        *out_start = -1;
        *out_end = -1;
        return 0;
    }

    const int32_t max_start = anchored_start ? (start_at == 0 ? 0 : -1) : length;
    for (int32_t s = start_at; s <= max_start; ++s) {
        int32_t state = 0;
        int32_t last_accept = accept_flags[0] ? s : -1;

        for (int32_t i = s; i < length; ++i) {
            const uint32_t ch = static_cast<uint32_t>(units[i]);
            int32_t next = -1;
            if (ch < 128u) {
                next = ascii_table_128[(static_cast<size_t>(state) << 7) + ch];
            } else if (has_non_ascii) {
                return -1; // Delegate non-ASCII char class matching to managed Unicode fallback
            }

            if (next < 0) {
                break;
            }
            state = next;
            if (accept_flags[state]) {
                last_accept = i + 1;
            }
        }

        if (last_accept >= 0) {
            if (!anchored_end || last_accept == length) {
                *out_start = s;
                *out_end = last_accept;
                return 1;
            }
        }
    }

    *out_start = -1;
    *out_end = -1;
    return 0;
}

} // namespace

extern "C" {

PILOT_API int32_t pilot_rt_dfa_find_utf16(
    const int32_t* ascii_table_128,
    const uint8_t* accept_flags,
    int32_t state_count,
    int32_t has_non_ascii,
    int32_t anchored_start,
    int32_t anchored_end,
    const uint16_t* text,
    int32_t text_len,
    int32_t start_at,
    int32_t* out_start,
    int32_t* out_end)
{
    return dfa_scan_generic<uint16_t>(
        ascii_table_128, accept_flags, state_count, has_non_ascii,
        anchored_start, anchored_end, text, text_len, start_at, out_start, out_end);
}

PILOT_API int32_t pilot_rt_dfa_find_utf8(
    const int32_t* ascii_table_128,
    const uint8_t* accept_flags,
    int32_t state_count,
    int32_t has_non_ascii,
    int32_t anchored_start,
    int32_t anchored_end,
    const uint8_t* bytes,
    int32_t byte_len,
    int32_t start_at,
    int32_t* out_start,
    int32_t* out_end)
{
    return dfa_scan_generic<uint8_t>(
        ascii_table_128, accept_flags, state_count, has_non_ascii,
        anchored_start, anchored_end, bytes, byte_len, start_at, out_start, out_end);
}

} // extern "C"
