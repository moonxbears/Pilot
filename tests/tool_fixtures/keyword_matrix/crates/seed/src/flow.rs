// Flow receptors (this comment is not code).
pub fn lower_receptor(kind: u32, sub: u32) -> u32 {
    match (kind, sub) {
        (15, 0) => lower_orbit(),
        (15, 1) => 7,
        (16, 0) => 9,
        _ => 0,
    }
}

fn lower_orbit() -> u32 {
    let turns = 3;
    turns * 2
}
