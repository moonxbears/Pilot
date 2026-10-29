// Code transpiled & optimized from Pilot to Modern C++ (C++20/23)
#include <iostream>
#include <string>
#include <vector>
#include <cstdint>
#include <cassert>
#include <cmath>
#include <optional>
#include <variant>

enum class Priority {
    Low = 1,
    Medium = 2,
    High = 3,
};

struct Circle {
    double r;
};
struct Rect {
    double w;
    double h;
};
using Shape = std::variant<Circle, Rect>;

inline int64_t i64_doubled(const int64_t& self_val) {
    return self_val * 2;
}

class Counter {
public:
    int64_t step(int64_t x, int64_t delta) {
        return x + delta;
    }
};

constexpr int64_t gcd(int64_t a, int64_t b) {
    int64_t x = a;
    int64_t y = b;
    while (y != 0) {
        const int64_t rem = x % y;
        x = y;
        y = rem;
    }
    return x;
}

constexpr int64_t compute_weighted_sum(int64_t limit, int64_t scale) {
    const int64_t base_bonus = (limit + 2) * scale;
    int64_t total = base_bonus;
    for (int64_t i = 1; i <= limit; ++i) {
        if (i % 2 == 0) {
            total += i * scale;
        } else if (i == 5) {
            total += 10;
        } else {
            total += i;
        }
    }
    return total;
}

int main() {
    constexpr int64_t BASE_SCALE = 3;
    const int64_t g = gcd(48, 18);
    const int64_t weighted = compute_weighted_sum(5, BASE_SCALE);
    const auto squares = ([&]() { std::vector<int64_t> __res; for (int64_t x = 1; x <= 6; ++x) { if (!(x > 2)) continue; __res.push_back(x * x); } return __res; })();
    assert(g == 6);
    assert(weighted == 53);
    std::cout << "GCD=" << (g) << ", Weighted=" << (weighted) << '\n';
    return 0;
}
