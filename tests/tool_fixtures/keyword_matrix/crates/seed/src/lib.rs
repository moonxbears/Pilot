pub fn run_all(k: u32, s: u32) -> u32 {
    lower_receptor(k, s) + lower_receptor(s, k) + lower_receptor(k, k)
}

pub fn is_rec(kind: u32) -> bool {
    kind == 16
}

pub fn tether_ready() -> bool {
    is_rec(16)
}
