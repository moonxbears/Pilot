//! Milestone 58 Rigorous Uncheatable Verification Suite:
//! Runtime Purification, Real Crypto, Wire DB & Polyglot — Retire `pilot-polyglot`.
//!
//! ============================================================================
//! FROZEN-RUST RUNNER CONTRACT (DO NOT MODIFY THIS TEST FILE OR RUST CRATES)
//! ============================================================================
//! All tests in this file must pass simultaneously using ONLY edits to `.pilot`
//! files (`selfhost/ffi_bindgen.pilot`, `stdlib/crypto/*.pilot`,
//! `stdlib/core/{reactive,effects}.pilot`, `stdlib/http/http.pilot`,
//! `stdlib/polyglot/*.pilot`, `stdlib/db/*.pilot`).
//!
//! Required `.pilot` entrypoints & behavioral contracts:
//!
//! 1. `selfhost/ffi_bindgen.pilot` (`<= 1,500` LOC):
//!    - `pub fn generate_ffi_binding(lib_name: str, symbol_name: str, param_name: str, param_reg: i64, param_bits: i64, ret_reg: i64, ret_bits: i64) -> FfiBindingBundle`
//!      (fields `.symbol_name: str`, `.c_prototype: str`, `.csharp_library_import: str`)
//!    - `pub fn generate_arena_c_ffi_header() -> str`
//!      Queries mounted `AstArena` + `PhysicsRegistry` (`ast_*`, `layout_*`, `sym_str`)
//!      to emit a valid `#include <stdint.h>` / `#include <stdbool.h>` C11 header
//!      declaring `typedef struct { ... } <StructName>;` (ordered by `layout_field_offset`)
//!      and `extern <ret_c_type> <fn_name>(...);` prototypes for `pub fn` declarations.
//!
//! 2. `stdlib/crypto/hash.pilot` (`<= 1,500` LOC):
//!    - `pub fn sha256_hex(msg: str) -> str` (64-char lowercase hex RFC 6234 SHA-256)
//!    - `pub fn sha512_hex(msg: str) -> str` (128-char lowercase hex RFC 6234 SHA-512)
//!    - `pub fn blake3_hex(msg: str) -> str` (64-char lowercase hex spec BLAKE3, supporting multi-block & multi-chunk Merkle parent folding)
//!    - `pub fn hmac_sha256_hex(key: str, msg: str) -> str` (64-char lowercase hex RFC 2104 HMAC-SHA256 for keys both `<= 64` and `> 64` bytes)
//!
//! 3. `stdlib/crypto/ed25519.pilot` (`<= 1,500` LOC):
//!    - `pub fn ed25519_public_key_hex(seed32_hex: str) -> str` (64-char lowercase hex 32-byte RFC 8032 public key)
//!    - `pub fn ed25519_sign_hex(seed32_hex: str, msg_hex: str) -> str` (128-char lowercase hex 64-byte `R || S` RFC 8032 signature)
//!    - `pub fn ed25519_verify_hex(pk32_hex: str, msg_hex: str, sig64_hex: str) -> bool`
//!      Full RFC 8032 Curve25519 / Edwards verification, including strict canonical `S < L` enforcement (`S >= L` MUST return `false`).
//!
//! 4. `stdlib/core/reactive.pilot` & `stdlib/core/effects.pilot` (`<= 1,500` LOC each):
//!    - `reactive.pilot`:
//!      - `pub fn rx_graph_new() -> i64`
//!      - `pub fn rx_source(g: i64, initial_val: i64) -> i64`
//!      - `pub fn rx_derived(g: i64, dep_a: i64, dep_b: i64, op_kind: i64, const_arg: i64) -> i64`
//!        (`op_kind`: `1 => dep_a * const_arg`, `2 => dep_a + const_arg`, `3 => dep_a + dep_b`)
//!      - `pub fn rx_set(g: i64, source_id: i64, new_val: i64) -> i64`
//!      - `pub fn rx_read(g: i64, node_id: i64) -> i64`
//!      - `pub fn rx_recompute_count(g: i64, node_id: i64) -> i64`
//!        (Glitch-free push-pull diamond evaluation: setting `S` to an unchanged value causes `0` recomputations; setting `S` to a new value recomputes diamond sink `D` at most `1` time per transaction/read.)
//!    - `effects.pilot`:
//!      - `pub fn effect_store_new() -> i64`
//!      - `pub fn effect_store_put(store: i64, key: str, val: str) -> i64`
//!      - `pub fn effect_store_get(store: i64, key: str) -> str` (returns `""` if missing/deleted)
//!      - `pub fn effect_sandbox_begin(store: i64) -> i64`
//!      - `pub fn effect_sandbox_put(sb: i64, key: str, val: str) -> i64`
//!      - `pub fn effect_sandbox_delete(sb: i64, key: str) -> i64`
//!      - `pub fn effect_sandbox_get(sb: i64, key: str) -> str`
//!      - `pub fn effect_sandbox_rollback(sb: i64) -> i64`
//!      - `pub fn effect_sandbox_commit(sb: i64, store: i64) -> i64`
//!
//! 5. `stdlib/http/http.pilot` (`<= 1,500` LOC):
//!    - Uses raw TCP engine syscalls (`__engine("tcp_connect", "host:port") -> i64`,
//!      `__engine("tcp_send", fd, data) -> i64`, `__engine("tcp_recv", fd, max_len) -> str`,
//!      `__engine("tcp_close", fd) -> i64`).
//!    - `pub fn http_get(url: str) -> HttpResponse` (fields `.status: i64`, `.body: str`, `.headers_raw: str`)
//!    - `pub fn http_post(url: str, content_type: str, body: str) -> HttpResponse`
//!    - `pub fn http_header_get(resp: HttpResponse, header_name: str) -> str` (case-insensitive header lookup; decodes `Transfer-Encoding: chunked` bodies as well as `Content-Length` bodies).
//!
//! 6. `stdlib/polyglot/{shm,marshal,bridge}.pilot` (`<= 1,500` LOC each):
//!    - `shm.pilot`:
//!      - `pub fn create_shm_owned_buffer(base_addr: i64, byte_capacity: i64, alignment: i64) -> ShmOwnedBuffer`
//!        (`.base_addr: i64`, `.byte_capacity: i64`, `.alignment: i64`, `.lattice_bits: i64` == `2`)
//!      - `pub fn project_shm_borrowed_view(buf: ShmOwnedBuffer, elem_offset: i64, elem_count: i64, elem_size: i64) -> ShmBorrowedView`
//!        (`.base_addr: i64`, `.byte_len: i64`, `.lattice_bits: i64` == `1`, `.valid: bool`; `valid == false` and `byte_len == 0` on negative/out-of-bounds slices!)
//!    - `marshal.pilot`:
//!      - `pub fn json_parse(src: str) -> i64`
//!      - `pub fn json_get_str(doc: i64, dot_path: str) -> str`
//!      - `pub fn json_get_int(doc: i64, dot_path: str) -> i64`
//!      - `pub fn json_get_bool(doc: i64, dot_path: str) -> bool`
//!      - `pub fn json_array_len(doc: i64, dot_path: str) -> i64`
//!      - `pub fn json_stringify(doc: i64) -> str`
//!    - `bridge.pilot`:
//!      - Uses `__engine("host_spawn_eval", runtime_bin: str, code: str) -> str`
//!      - `pub fn polyglot_eval_json(lang: str, json_bindings: str, user_expr: str) -> str`
//!        (`lang` in `"python"` | `"node"`: unmarshals `json_bindings` into variables in the target script, evaluates `user_expr`, and returns JSON-encoded output).
//!
//! 7. `stdlib/db/{sql_engine,postgres_wire,mysql_wire}.pilot` (`<= 1,500` LOC each):
//!    - `sql_engine.pilot`:
//!      - `pub fn sql_db_new() -> i64`
//!      - `pub fn sql_exec(db: i64, sql: str) -> str`
//!        Supports `CREATE TABLE`, `INSERT INTO`, `SELECT col1, col2 FROM ... [JOIN ... ON ...] [WHERE ...] [ORDER BY col ASC|DESC]`,
//!        `UPDATE ... SET ... WHERE ...`, and `DELETE FROM ... WHERE ...`.
//!        For `SELECT`, returns `\n`-joined rows with `|`-joined column values (empty string `""` if 0 rows).
//!    - `postgres_wire.pilot`:
//!      - `pub fn pg_encode_startup_hex(user: str, database: str) -> str`
//!      - `pub fn pg_decode_startup_info(frame_hex: str) -> str` (returns `"<user>|<database>"`, validating protocol `196608` and exact BE `u32` frame length)
//!      - `pub fn pg_encode_query_hex(sql: str) -> str`
//!      - `pub fn pg_decode_query_sql(frame_hex: str) -> str`
//!      - `pub fn pg_encode_data_row_hex(col0: str, col1: str) -> str`
//!      - `pub fn pg_decode_data_row_pipe(frame_hex: str) -> str` (returns `"<col0>|<col1>"`)
//!    - `mysql_wire.pilot`:
//!      - `pub fn mysql_encode_com_query_hex(seq_id: i64, sql: str) -> str`
//!      - `pub fn mysql_decode_com_query(frame_hex: str) -> str` (returns `"<seq_id>|<sql>"`, validating 3-byte LE payload length and `0x03` command byte)

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pilot_analysis::{AnalysisPipeline, handle_engine_intrinsic, PhysicsRegistry};
use pilot_parser::parse;
use pilot_rt::PilotVal;
use pilot_vm::{BytecodeCompiler, PilVirtualMachine};

#[path = "common/anticheat.rs"]
mod anticheat;

const ANTICHEAT_SPEC: anticheat::MilestoneAuditSpec = anticheat::MilestoneAuditSpec {
    milestone_id: 58,
    test_file_rel: "crates/pilot-analysis/tests/milestone58_runtime_crypto_wire_db_and_polyglot.rs",
    expected_test_count: 7,
    files: &[
        anticheat::FileAuditRule {
            path: "selfhost/ffi_bindgen.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["generate_ffi_binding", "generate_arena_c_ffi_header"],
            required_code_markers: &["__engine"],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/crypto/hash.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "sha256_hex",
                "sha512_hex",
                "blake3_hex",
                "hmac_sha256_hex",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["14695981039346656037", "0xcbf29ce484222325", "host_spawn_eval"],
            forbidden_string_substrings: &[
                "sha256_computed",
                "blake3_computed",
                "openssl",
                "hashlib",
                "libcrypto",
                "python3",
            ],
        },
        anticheat::FileAuditRule {
            path: "stdlib/crypto/ed25519.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "ed25519_public_key_hex",
                "ed25519_sign_hex",
                "ed25519_verify_hex",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["2147483647", "host_spawn_eval"],
            forbidden_string_substrings: &[
                "openssl",
                "libsodium",
                "libcrypto",
                "crypto.sign",
                "crypto.verify",
            ],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/reactive.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "rx_graph_new",
                "rx_source",
                "rx_derived",
                "rx_set",
                "rx_read",
                "rx_recompute_count",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/core/effects.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "effect_store_new",
                "effect_store_put",
                "effect_store_get",
                "effect_sandbox_begin",
                "effect_sandbox_put",
                "effect_sandbox_delete",
                "effect_sandbox_get",
                "effect_sandbox_rollback",
                "effect_sandbox_commit",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/http/http.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["http_get", "http_post", "http_header_get"],
            required_code_markers: &["tcp_connect", "tcp_send", "tcp_recv"],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &["curl", "urllib", "requests"],
        },
        anticheat::FileAuditRule {
            path: "stdlib/polyglot/shm.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["create_shm_owned_buffer", "project_shm_borrowed_view"],
            required_code_markers: &[],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/polyglot/marshal.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "json_parse",
                "json_get_str",
                "json_get_int",
                "json_get_bool",
                "json_array_len",
                "json_stringify",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/polyglot/bridge.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["polyglot_eval_json"],
            required_code_markers: &["host_spawn_eval"],
            forbidden_code_markers: &[],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/db/sql_engine.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["sql_db_new", "sql_exec"],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &["sqlite3", "libsqlite", "item_"],
        },
        anticheat::FileAuditRule {
            path: "stdlib/db/postgres_wire.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &[
                "pg_encode_startup_hex",
                "pg_decode_startup_info",
                "pg_encode_query_hex",
                "pg_decode_query_sql",
                "pg_encode_data_row_hex",
                "pg_decode_data_row_pipe",
            ],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
        anticheat::FileAuditRule {
            path: "stdlib/db/mysql_wire.pilot",
            max_lines: 1500,
            name_blind: false,
            required_pub_fns: &["mysql_encode_com_query_hex", "mysql_decode_com_query"],
            required_code_markers: &[],
            forbidden_code_markers: &["host_spawn_eval"],
            forbidden_string_substrings: &[],
        },
    ],
    required_group_pub_fns: &[],
};

struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn from_entropy() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x9E37_79B9_7F4A_7C15);
        let pid = std::process::id() as u64;
        Self {
            state: nanos ^ pid.rotate_left(19) ^ 0xD1B5_4A32_D192_ED03,
        }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn range_i64(&mut self, lo: i64, hi_inclusive: i64) -> i64 {
        let span = (hi_inclusive - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    fn ascii_payload(&mut self, len: usize) -> String {
        const ALPHABET: &[u8] =
            b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-:/.,+=@#%~";
        let mut out = String::with_capacity(len);
        for _ in 0..len {
            let idx = (self.next_u64() as usize) % ALPHABET.len();
            out.push(ALPHABET[idx] as char);
        }
        out
    }

    fn hex_bytes(&mut self, byte_len: usize) -> String {
        let mut out = String::with_capacity(byte_len * 2);
        for _ in 0..byte_len {
            let b = (self.next_u64() & 0xFF) as u8;
            out.push_str(&format!("{:02x}", b));
        }
        out
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn escape_pilot_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

// ============================================================================
// Pure-Rust Official BLAKE3 Reference Oracle (Single-Chunk & Multi-Chunk Tree)
// ============================================================================
const B3_IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A,
    0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];
const B3_PERM: [usize; 16] = [2, 6, 3, 10, 7, 0, 4, 13, 1, 11, 12, 5, 9, 14, 15, 8];

fn b3_g(s: &mut [u32; 16], a: usize, b: usize, c: usize, d: usize, mx: u32, my: u32) {
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(mx);
    s[d] = (s[d] ^ s[a]).rotate_right(16);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(12);
    s[a] = s[a].wrapping_add(s[b]).wrapping_add(my);
    s[d] = (s[d] ^ s[a]).rotate_right(8);
    s[c] = s[c].wrapping_add(s[d]);
    s[b] = (s[b] ^ s[c]).rotate_right(7);
}

fn b3_compress(cv: &[u32; 8], block: &[u8; 64], counter: u64, block_len: u32, flags: u32) -> [u32; 16] {
    let mut m = [0u32; 16];
    for i in 0..16 {
        m[i] = u32::from_le_bytes([
            block[4 * i],
            block[4 * i + 1],
            block[4 * i + 2],
            block[4 * i + 3],
        ]);
    }
    let mut s = [
        cv[0], cv[1], cv[2], cv[3], cv[4], cv[5], cv[6], cv[7],
        B3_IV[0], B3_IV[1], B3_IV[2], B3_IV[3],
        counter as u32, (counter >> 32) as u32, block_len, flags,
    ];
    for r in 0..7 {
        b3_g(&mut s, 0, 4, 8, 12, m[0], m[1]);
        b3_g(&mut s, 1, 5, 9, 13, m[2], m[3]);
        b3_g(&mut s, 2, 6, 10, 14, m[4], m[5]);
        b3_g(&mut s, 3, 7, 11, 15, m[6], m[7]);
        b3_g(&mut s, 0, 5, 10, 15, m[8], m[9]);
        b3_g(&mut s, 1, 6, 11, 12, m[10], m[11]);
        b3_g(&mut s, 2, 7, 8, 13, m[12], m[13]);
        b3_g(&mut s, 3, 4, 9, 14, m[14], m[15]);
        if r < 6 {
            let mut next_m = [0u32; 16];
            for i in 0..16 {
                next_m[i] = m[B3_PERM[i]];
            }
            m = next_m;
        }
    }
    for i in 0..8 {
        s[i] ^= s[i + 8];
        s[i + 8] ^= cv[i];
    }
    s
}

fn b3_first8(s: [u32; 16]) -> [u32; 8] {
    [s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]
}

fn b3_parent_block(left: &[u32; 8], right: &[u32; 8]) -> [u8; 64] {
    let mut blk = [0u8; 64];
    for i in 0..8 {
        blk[4 * i..4 * i + 4].copy_from_slice(&left[i].to_le_bytes());
        blk[32 + 4 * i..32 + 4 * i + 4].copy_from_slice(&right[i].to_le_bytes());
    }
    blk
}

fn b3_chunk_final_node(chunk: &[u8], counter: u64) -> ([u32; 8], [u8; 64], u64, u32, u32) {
    let mut cv = B3_IV;
    let nblocks = if chunk.is_empty() { 1 } else { (chunk.len() + 63) / 64 };
    for b in 0..nblocks {
        let start = b * 64;
        let end = ((b + 1) * 64).min(chunk.len());
        let sub = if start < chunk.len() { &chunk[start..end] } else { &[] };
        let mut blk = [0u8; 64];
        blk[..sub.len()].copy_from_slice(sub);
        let flags = (if b == 0 { 1 } else { 0 }) | (if b + 1 == nblocks { 2 } else { 0 });
        if b + 1 == nblocks {
            return (cv, blk, counter, sub.len() as u32, flags);
        }
        cv = b3_first8(b3_compress(&cv, &blk, counter, sub.len() as u32, flags));
    }
    unreachable!()
}

fn blake3_oracle_hex(data: &[u8]) -> String {
    let total_chunks = if data.is_empty() { 1 } else { (data.len() + 1023) / 1024 };
    let words_to_hex = |words: &[u32; 8]| -> String {
        let mut out = String::with_capacity(64);
        for w in words {
            for b in w.to_le_bytes() {
                out.push_str(&format!("{:02x}", b));
            }
        }
        out
    };
    if total_chunks == 1 {
        let (cv, blk, ctr, blen, fl) = b3_chunk_final_node(data, 0);
        let st = b3_first8(b3_compress(&cv, &blk, ctr, blen, fl | 8));
        return words_to_hex(&st);
    }
    let mut stack: Vec<([u32; 8], u32)> = Vec::new();
    for idx in 0..total_chunks {
        let ch_start = idx * 1024;
        let ch_end = ((idx + 1) * 1024).min(data.len());
        let (cv, blk, ctr, blen, fl) = b3_chunk_final_node(&data[ch_start..ch_end], idx as u64);
        let mut cur = (b3_first8(b3_compress(&cv, &blk, ctr, blen, fl)), 0u32);
        if idx + 1 < total_chunks {
            while stack.last().map_or(false, |&(_, h)| h == cur.1) {
                let (left, h) = stack.pop().unwrap();
                let pblk = b3_parent_block(&left, &cur.0);
                cur = (b3_first8(b3_compress(&B3_IV, &pblk, 0, 64, 4)), h + 1);
            }
            stack.push(cur);
        } else {
            while stack.len() > 1 {
                let (left, _) = stack.pop().unwrap();
                let pblk = b3_parent_block(&left, &cur.0);
                cur.0 = b3_first8(b3_compress(&B3_IV, &pblk, 0, 64, 4));
            }
            let (left, _) = stack.pop().unwrap();
            let pblk = b3_parent_block(&left, &cur.0);
            let root = b3_first8(b3_compress(&B3_IV, &pblk, 0, 64, 4 | 8));
            return words_to_hex(&root);
        }
    }
    unreachable!()
}

fn python_hash_oracle(algo: &str, key_opt: Option<&str>, msg: &str) -> String {
    let py = r#"
import hashlib, hmac, json, sys
req = json.loads(sys.stdin.read())
algo = req["algo"]
msg = req["msg"].encode("utf-8")
if algo == "sha256":
    print(hashlib.sha256(msg).hexdigest())
elif algo == "sha512":
    print(hashlib.sha512(msg).hexdigest())
elif algo == "hmac_sha256":
    key = req["key"].encode("utf-8")
    print(hmac.new(key, msg, hashlib.sha256).hexdigest())
"#;
    let payload = format!(
        "{{\"algo\":\"{}\",\"key\":\"{}\",\"msg\":\"{}\"}}",
        algo,
        escape_pilot_str(key_opt.unwrap_or("")),
        escape_pilot_str(msg)
    );
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(py)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("python3 must be installed for crypto reference verification");
    child.stdin.as_mut().unwrap().write_all(payload.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "python3 hash oracle failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

fn node_ed25519_sign_oracle(seed32_hex: &str, msg_hex: &str) -> (String, String) {
    let js = r#"
const crypto = require("crypto");
const fs = require("fs");
const req = JSON.parse(fs.readFileSync(0, "utf8"));
const seed = Buffer.from(req.seed, "hex");
const msg = Buffer.from(req.msg, "hex");
const pkcs8 = Buffer.concat([Buffer.from("302e020100300506032b657004220420", "hex"), seed]);
const priv = crypto.createPrivateKey({ key: pkcs8, format: "der", type: "pkcs8" });
const pubDer = crypto.createPublicKey(priv).export({ format: "der", type: "spki" });
const pk = pubDer.subarray(12).toString("hex");
const sig = crypto.sign(null, msg, priv).toString("hex");
process.stdout.write(pk + "|" + sig);
"#;
    let payload = format!("{{\"seed\":\"{}\",\"msg\":\"{}\"}}", seed32_hex, msg_hex);
    let mut child = Command::new("node")
        .arg("-e")
        .arg(js)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("node must be installed for Ed25519 reference verification");
    child.stdin.as_mut().unwrap().write_all(payload.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "node ed25519 sign oracle failed: {}", String::from_utf8_lossy(&out.stderr));
    let s = String::from_utf8(out.stdout).unwrap();
    let (pk, sig) = s.split_once('|').unwrap();
    (pk.to_string(), sig.to_string())
}

fn node_ed25519_verify_oracle(pk32_hex: &str, msg_hex: &str, sig64_hex: &str) -> bool {
    let js = r#"
const crypto = require("crypto");
const fs = require("fs");
const req = JSON.parse(fs.readFileSync(0, "utf8"));
const pk = Buffer.from(req.pk, "hex");
const msg = Buffer.from(req.msg, "hex");
const sig = Buffer.from(req.sig, "hex");
const spki = Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), pk]);
const pubKey = crypto.createPublicKey({ key: spki, format: "der", type: "spki" });
process.stdout.write(crypto.verify(null, msg, pubKey, sig) ? "1" : "0");
"#;
    let payload = format!("{{\"pk\":\"{}\",\"msg\":\"{}\",\"sig\":\"{}\"}}", pk32_hex, msg_hex, sig64_hex);
    let mut child = Command::new("node")
        .arg("-e")
        .arg(js)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.as_mut().unwrap().write_all(payload.as_bytes()).unwrap();
    let out = child.wait_with_output().unwrap();
    out.status.success() && String::from_utf8_lossy(&out.stdout).trim() == "1"
}

/// Computes `S + L` in 32-byte little-endian hex, producing a non-canonical scalar `>= L`
/// where `L = 2^252 + 27742317777372353535851937790883648493`.
fn add_ed25519_order_l_to_sig_s(sig64_hex: &str) -> String {
    const ORDER_L_LE: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
        0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10,
    ];
    let r_hex = &sig64_hex[0..64];
    let s_hex = &sig64_hex[64..128];
    let mut s_bytes = [0u8; 32];
    for i in 0..32 {
        s_bytes[i] = u8::from_str_radix(&s_hex[2 * i..2 * i + 2], 16).unwrap();
    }
    let mut carry = 0u16;
    for i in 0..32 {
        let sum = (s_bytes[i] as u16) + (ORDER_L_LE[i] as u16) + carry;
        s_bytes[i] = (sum & 0xFF) as u8;
        carry = sum >> 8;
    }
    let mut out = String::with_capacity(128);
    out.push_str(r_hex);
    for b in s_bytes {
        out.push_str(&format!("{:02x}", b));
    }
    out
}

fn flip_hex_bit(hex_str: &str, byte_idx: usize, bit_mask: u8) -> String {
    let mut out = String::with_capacity(hex_str.len());
    for i in 0..(hex_str.len() / 2) {
        let mut b = u8::from_str_radix(&hex_str[2 * i..2 * i + 2], 16).unwrap();
        if i == byte_idx {
            b ^= bit_mask;
        }
        out.push_str(&format!("{:02x}", b));
    }
    out
}

#[test]
pub fn test_58_1_ffi_bindgen_clang_c11_header_and_rt_getrandom_dlopen() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_1_ffi_bindgen_clang_c11_header_and_rt_getrandom_dlopen",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    // 1. OS CSPRNG syscall & DynamicLibraryHandle in purified `pilot-rt`
    let mut e1 = [0u8; 32];
    let mut e2 = [0u8; 32];
    let n1 = unsafe { pilot_rt::pilot_rt_os_getrandom(e1.as_mut_ptr(), e1.len()) };
    let n2 = unsafe { pilot_rt::pilot_rt_os_getrandom(e2.as_mut_ptr(), e2.len()) };
    assert_eq!(n1, 32);
    assert_eq!(n2, 32);
    assert_ne!(e1, [0u8; 32], "CSPRNG returned all-zero buffer");
    assert_ne!(e1, e2, "Two consecutive 256-bit OS CSPRNG draws collided");

    if let Ok(libc_handle) = pilot_rt::DynamicLibraryHandle::open("libc.so.6") {
        let sym = libc_handle
            .resolve_symbol("strlen")
            .expect("dlsym('strlen') must succeed on libc.so.6");
        assert!(!sym.is_null());
    }

    // 2. Mount randomized AST + PhysicsRegistry and generate C11 FFI header in Pilot
    let ffi_bg_src = std::fs::read_to_string(root.join("selfhost/ffi_bindgen.pilot"))
        .expect("selfhost/ffi_bindgen.pilot must exist");
    assert!(ffi_bg_src.lines().count() <= 1500);

    let tag = rng.next_u64() & 0xFFFF_FFFF;
    let struct_name = format!("PacketFrame_{:08x}", tag);
    let field_a = format!("seq_no_{:04x}", tag & 0xFFFF);
    let field_b = format!("gain_{:04x}", (tag >> 16) & 0xFFFF);
    let fn_name = format!("pilot_ffi_step_{:08x}", tag);

    let target_pilot = format!(
        "[Bits(64, 2)] struct MyI64 {{}}\n\
         [Bits(64, 4)] struct MyF64 {{}}\n\
         pub struct {struct_name} {{\n\
             pub {field_a}: MyI64;\n\
             pub {field_b}: MyF64;\n\
         }}\n\
         pub fn {fn_name}(arg_in: MyI64, scale_in: MyF64) -> MyI64 {{\n\
             return arg_in;\n\
         }}\n"
    );

    let target_arena = parse(&target_pilot);
    assert!(
        target_arena.diagnostics.is_empty(),
        "Target FFI Pilot source must parse cleanly: {:?}",
        target_arena.diagnostics
    );
    let analysis = AnalysisPipeline::analyze(&target_arena, target_arena.root);

    let driver = format!(
        "{}\n\
         let b = generate_ffi_binding(\"pilot_rt\", \"{}\", \"x_arg\", 2, 64, 4, 64);\n\
         let hdr = generate_arena_c_ffi_header();\n\
         let __out = b.c_prototype + \"\\n===HDR===\\n\" + hdr;\n\
         __out;\n",
        ffi_bg_src, fn_name
    );

    let drv_arena = parse(&driver);
    assert!(drv_arena.diagnostics.is_empty(), "ffi_bindgen parse diagnostics: {:?}", drv_arena.diagnostics);
    let mut reg = analysis.physics.clone();
    reg.source_text = target_pilot.clone();
    let reg_ptr = &mut reg as *mut PhysicsRegistry;
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(move |op, args| {
        handle_engine_intrinsic(unsafe { &mut *reg_ptr }, op, args)
    }));
    vm.load_binary(BytecodeCompiler::compile(&drv_arena, drv_arena.root));
    let out = vm.run().to_string_repr();

    let (proto, hdr) = out
        .split_once("\n===HDR===\n")
        .expect("Expected prototype + header output");
    assert!(
        proto.contains(&fn_name) && proto.contains("double") && proto.contains("int64_t"),
        "Unexpected C prototype: {}",
        proto
    );
    assert!(
        hdr.contains(&struct_name) && hdr.contains(&field_a) && hdr.contains(&field_b) && hdr.contains(&fn_name),
        "Generated C11 FFI header must declare mounted struct fields and pub fn:\n{}",
        hdr
    );

    // Syntax-check generated header with `clang -x c -std=c11 -Wall -Werror -fsyntax-only`
    let mut clang = Command::new("clang")
        .args(["-x", "c", "-std=c11", "-Wall", "-Werror", "-fsyntax-only", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("clang must be installed");
    clang.stdin.as_mut().unwrap().write_all(hdr.as_bytes()).unwrap();
    let clang_out = clang.wait_with_output().unwrap();
    assert!(
        clang_out.status.success(),
        "clang C11 syntax check failed on generated FFI header:\n{}\nHeader:\n{}",
        String::from_utf8_lossy(&clang_out.stderr),
        hdr
    );
    guard.finish();
}

#[test]
pub fn test_58_2a_sha256_sha512_blake3_and_hmac_randomized_boundary_oracles() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_2a_sha256_sha512_blake3_and_hmac_randomized_boundary_oracles",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();
    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot"))
        .expect("stdlib/crypto/hash.pilot must exist");
    assert!(hash_src.lines().count() <= 1500);

    // Boundary lengths covering empty, short, SHA-256 55/56/64B block splits,
    // SHA-512 111/112/128B block splits, and BLAKE3 64/65/1024/1025/2050B 1-, 2-, and 3-chunk Merkle trees!
    let lengths = [0usize, 1, 3, 55, 56, 63, 64, 65, 111, 112, 127, 128, 129, 1024, 1025, 2050];
    let msgs: Vec<String> = lengths.iter().map(|&n| rng.ascii_payload(n)).collect();
    let short_key = rng.ascii_payload(24); // <= 64B HMAC key
    let long_key = rng.ascii_payload(91); // > 64B HMAC key (forces SHA-256 key compression!)

    let mut calls = String::new();
    calls.push_str("var __acc = \"\";\n");
    for (idx, m) in msgs.iter().enumerate() {
        let esc = escape_pilot_str(m);
        calls.push_str(&format!(
            "let s256_{idx} = sha256_hex(\"{esc}\");\n\
             let s512_{idx} = sha512_hex(\"{esc}\");\n\
             let b3_{idx} = blake3_hex(\"{esc}\");\n\
             let hm_s_{idx} = hmac_sha256_hex(\"{sk}\", \"{esc}\");\n\
             let hm_l_{idx} = hmac_sha256_hex(\"{lk}\", \"{esc}\");\n\
             __acc = __acc + s256_{idx} + \"|\" + s512_{idx} + \"|\" + b3_{idx} + \"|\" + hm_s_{idx} + \"|\" + hm_l_{idx} + \"\\n\";\n",
            idx = idx,
            esc = esc,
            sk = escape_pilot_str(&short_key),
            lk = escape_pilot_str(&long_key),
        ));
    }
    calls.push_str("__acc;\n");

    let script = format!("{}\n{}", hash_src, calls);
    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "hash.pilot parse diagnostics: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let out = vm.run().to_string_repr();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), msgs.len(), "Expected {} digest rows from hash.pilot", msgs.len());

    for (idx, m) in msgs.iter().enumerate() {
        let parts: Vec<&str> = lines[idx].split('|').collect();
        assert_eq!(parts.len(), 5, "Malformed digest row at len={}", m.len());
        let exp_s256 = python_hash_oracle("sha256", None, m);
        let exp_s512 = python_hash_oracle("sha512", None, m);
        let exp_b3 = blake3_oracle_hex(m.as_bytes());
        let exp_hms = python_hash_oracle("hmac_sha256", Some(&short_key), m);
        let exp_hml = python_hash_oracle("hmac_sha256", Some(&long_key), m);

        assert_eq!(parts[0], exp_s256, "SHA-256 mismatch at byte length {}", m.len());
        assert_eq!(parts[1], exp_s512, "SHA-512 mismatch at byte length {}", m.len());
        assert_eq!(parts[2], exp_b3, "BLAKE3 mismatch at byte length {}", m.len());
        assert_eq!(parts[3], exp_hms, "HMAC-SHA256 (short key) mismatch at byte length {}", m.len());
        assert_eq!(parts[4], exp_hml, "HMAC-SHA256 (long >64B key) mismatch at byte length {}", m.len());
    }
    guard.finish();
}

#[test]
pub fn test_58_2b_ed25519_rfc8032_bidirectional_node_oracle_and_malleability_rejection() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_2b_ed25519_rfc8032_bidirectional_node_oracle_and_malleability_rejection",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();
    let hash_src = std::fs::read_to_string(root.join("stdlib/crypto/hash.pilot"))
        .expect("stdlib/crypto/hash.pilot must exist");
    let ed_src = std::fs::read_to_string(root.join("stdlib/crypto/ed25519.pilot"))
        .expect("stdlib/crypto/ed25519.pilot must exist");
    assert!(ed_src.lines().count() <= 1500);

    // Official RFC 8032 §7.1 Test Vector 1 & Test Vector 2
    let tv1_seed = "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60";
    let tv1_pk = "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    let tv1_sig = "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

    let tv2_seed = "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb";
    let tv2_msg = "72";
    let tv2_pk = "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    let tv2_sig = "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

    // Runtime random Seed A (signed in Pilot, verified in BOTH Pilot and Node)
    let rand_seed_a = rng.hex_bytes(32);
    let len_a = rng.range_i64(19, 95) as usize;
    let rand_msg_a = rng.hex_bytes(len_a);
    let (exp_pk_a, exp_sig_a) = node_ed25519_sign_oracle(&rand_seed_a, &rand_msg_a);

    // Runtime random Seed B (signed ONLY inside external `node` process! Pilot NEVER sees `rand_seed_b`!)
    // This makes shared-state sign->verify memoization in Pilot mathematically impossible.
    let rand_seed_b = rng.hex_bytes(32);
    let len_b = rng.range_i64(33, 140) as usize;
    let rand_msg_b = rng.hex_bytes(len_b);
    let (ext_pk_b, ext_sig_b) = node_ed25519_sign_oracle(&rand_seed_b, &rand_msg_b);

    // Adversarial forgeries against `(ext_pk_b, rand_msg_b, ext_sig_b)`:
    let bad_r = flip_hex_bit(&ext_sig_b, 7, 0x04);
    let bad_s = flip_hex_bit(&ext_sig_b, 41, 0x01);
    let bad_msg = flip_hex_bit(&rand_msg_b, 3, 0x80);
    let bad_pk = flip_hex_bit(&ext_pk_b, 12, 0x02);
    let non_canonical_s_plus_l = add_ed25519_order_l_to_sig_s(&ext_sig_b);

    let script = format!(
        "{hash_src}\n{ed_src}\n\
         let pk1 = ed25519_public_key_hex(\"{tv1_seed}\");\n\
         let sg1 = ed25519_sign_hex(\"{tv1_seed}\", \"\");\n\
         let pk2 = ed25519_public_key_hex(\"{tv2_seed}\");\n\
         let sg2 = ed25519_sign_hex(\"{tv2_seed}\", \"{tv2_msg}\");\n\
         let pka = ed25519_public_key_hex(\"{rand_seed_a}\");\n\
         let sga = ed25519_sign_hex(\"{rand_seed_a}\", \"{rand_msg_a}\");\n\
         let ok_a = ed25519_verify_hex(pka, \"{rand_msg_a}\", sga);\n\
         let ok_ext_b = ed25519_verify_hex(\"{ext_pk_b}\", \"{rand_msg_b}\", \"{ext_sig_b}\");\n\
         let rej_r = !ed25519_verify_hex(\"{ext_pk_b}\", \"{rand_msg_b}\", \"{bad_r}\");\n\
         let rej_s = !ed25519_verify_hex(\"{ext_pk_b}\", \"{rand_msg_b}\", \"{bad_s}\");\n\
         let rej_m = !ed25519_verify_hex(\"{ext_pk_b}\", \"{bad_msg}\", \"{ext_sig_b}\");\n\
         let rej_p = !ed25519_verify_hex(\"{bad_pk}\", \"{rand_msg_b}\", \"{ext_sig_b}\");\n\
         let rej_l = !ed25519_verify_hex(\"{ext_pk_b}\", \"{rand_msg_b}\", \"{non_canonical_s_plus_l}\");\n\
         let flags = if ok_a && ok_ext_b && rej_r && rej_s && rej_m && rej_p && rej_l {{ \"PASS\" }} else {{ \"FAIL\" }};\n\
         let __res = pk1 + \"|\" + sg1 + \"|\" + pk2 + \"|\" + sg2 + \"|\" + pka + \"|\" + sga + \"|\" + flags;\n\
         __res;\n"
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "ed25519.pilot parse diagnostics: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let res = vm.run().to_string_repr();
    let cols: Vec<&str> = res.split('|').collect();
    assert_eq!(cols.len(), 7, "Unexpected ed25519 result: {}", res);
    assert_eq!(cols[0], tv1_pk, "RFC 8032 TV1 public key mismatch");
    assert_eq!(cols[1], tv1_sig, "RFC 8032 TV1 signature mismatch");
    assert_eq!(cols[2], tv2_pk, "RFC 8032 TV2 public key mismatch");
    assert_eq!(cols[3], tv2_sig, "RFC 8032 TV2 signature mismatch");
    assert_eq!(cols[4], exp_pk_a, "Random Seed A Ed25519 public key mismatch vs Node OpenSSL");
    assert_eq!(cols[5], exp_sig_a, "Random Seed A Ed25519 signature mismatch vs Node OpenSSL");
    assert!(
        node_ed25519_verify_oracle(cols[4], &rand_msg_a, cols[5]),
        "Node OpenSSL rejected Pilot's Ed25519 signature"
    );
    assert_eq!(
        cols[6], "PASS",
        "Ed25519 external verification or forgery/S+L malleability rejection failed"
    );
    guard.finish();
}

#[test]
pub fn test_58_3_glitch_free_diamond_reactive_cow_effects_and_real_tcp_http11() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_3_glitch_free_diamond_reactive_cow_effects_and_real_tcp_http11",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let reactive_src = std::fs::read_to_string(root.join("stdlib/core/reactive.pilot")).unwrap();
    let effects_src = std::fs::read_to_string(root.join("stdlib/core/effects.pilot")).unwrap();
    let http_src = std::fs::read_to_string(root.join("stdlib/http/http.pilot")).unwrap();
    assert!(reactive_src.lines().count() <= 1500);
    assert!(effects_src.lines().count() <= 1500);
    assert!(http_src.lines().count() <= 1500);

    // 1. Real OS loopback TCP HTTP/1.1 server serving BOTH chunked GET and Content-Length POST
    let listener = pilot_rt::tcp_listen("127.0.0.1:0").expect("loopback tcp_listen must succeed");
    let port = listener.local_addr().unwrap().port();
    let get_path = format!("/v1/telemetry/{:x}", rng.next_u64() & 0xFFFF_FFFF);
    let post_path = format!("/v1/ingest/{:x}", rng.next_u64() & 0xFFFF_FFFF);
    let chunk_1 = rng.ascii_payload(19);
    let chunk_2 = rng.ascii_payload(31);
    let expected_get_body = format!("{}{}", chunk_1, chunk_2);
    let expected_trace_hdr = format!("trace-{:012x}", rng.next_u64() & 0xFFFF_FFFF_FFFF);
    let post_req_body = format!("{{\"seq\":{},\"tag\":\"{}\"}}", rng.range_i64(100, 9999), rng.ascii_payload(12));
    let post_resp_body = format!("{{\"accepted\":true,\"bytes\":{}}}", post_req_body.len());

    let captured_requests: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let cap_clone = Arc::clone(&captured_requests);
    let c1_srv = chunk_1.clone();
    let c2_srv = chunk_2.clone();
    let th_srv = expected_trace_hdr.clone();
    let pr_srv = post_resp_body.clone();

    let server_thread = std::thread::spawn(move || {
        for req_idx in 0..2 {
            let (mut stream, _) = listener.accept().expect("expected HTTP/1.1 TCP client connection");
            let mut buf = [0u8; 8192];
            let mut raw = Vec::new();
            loop {
                let n = stream.read(&mut buf).unwrap_or(0);
                if n == 0 {
                    break;
                }
                raw.extend_from_slice(&buf[..n]);
                if let Some(hdr_end) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    let hdr_str = String::from_utf8_lossy(&raw[..hdr_end]);
                    let mut content_len = 0usize;
                    for line in hdr_str.lines() {
                        if let Some((k, v)) = line.split_once(':') {
                            if k.trim().eq_ignore_ascii_case("content-length") {
                                content_len = v.trim().parse().unwrap_or(0);
                            }
                        }
                    }
                    if raw.len() >= hdr_end + 4 + content_len {
                        break;
                    }
                }
            }
            let req_text = String::from_utf8_lossy(&raw).to_string();
            cap_clone.lock().unwrap().push(req_text);

            if req_idx == 0 {
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nX-Pilot-Trace: {}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{}\r\n{:x}\r\n{}\r\n0\r\n\r\n",
                    th_srv,
                    c1_srv.len(),
                    c1_srv,
                    c2_srv.len(),
                    c2_srv
                );
                stream.write_all(resp.as_bytes()).unwrap();
            } else {
                let resp = format!(
                    "HTTP/1.1 201 Created\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    pr_srv.len(),
                    pr_srv
                );
                stream.write_all(resp.as_bytes()).unwrap();
            }
            let _ = stream.flush();
        }
    });

    // Randomized diamond graph parameters & CoW sandbox values
    let s0 = rng.range_i64(5, 30);
    let s1 = s0 + rng.range_i64(7, 40);
    let k1 = rng.range_i64(3, 9);
    let k2 = rng.range_i64(11, 50);
    let exp_d0 = s0 * k1 + (s0 + k2);
    let exp_d1 = s1 * k1 + (s1 + k2);

    let k_alpha = format!("k_alpha_{:x}", rng.next_u64() & 0xFFFF);
    let k_beta = format!("k_beta_{:x}", rng.next_u64() & 0xFFFF);
    let v_init = rng.ascii_payload(10);
    let v_upd = rng.ascii_payload(12);
    let v_new = rng.ascii_payload(11);

    let script = format!(
        "{reactive_src}\n{effects_src}\n{http_src}\n\
         let g = rx_graph_new();\n\
         let src = rx_source(g, {s0});\n\
         let left = rx_derived(g, src, 0, 1, {k1});\n\
         let right = rx_derived(g, src, 0, 2, {k2});\n\
         let sink = rx_derived(g, left, right, 3, 0);\n\
         let d0 = rx_read(g, sink);\n\
         let c_before = rx_recompute_count(g, sink);\n\
         let _noop = rx_set(g, src, {s0});\n\
         let d_same = rx_read(g, sink);\n\
         let c_noop = rx_recompute_count(g, sink) - c_before;\n\
         let _chg = rx_set(g, src, {s1});\n\
         let d1 = rx_read(g, sink);\n\
         let c_delta = rx_recompute_count(g, sink) - c_before;\n\
         \n\
         let st = effect_store_new();\n\
         let _p0 = effect_store_put(st, \"{k_alpha}\", \"{v_init}\");\n\
         let sb1 = effect_sandbox_begin(st);\n\
         let _s1 = effect_sandbox_put(sb1, \"{k_alpha}\", \"{v_upd}\");\n\
         let _s2 = effect_sandbox_put(sb1, \"{k_beta}\", \"{v_new}\");\n\
         let iso_before = effect_store_get(st, \"{k_alpha}\");\n\
         let sb_view = effect_sandbox_get(sb1, \"{k_alpha}\");\n\
         let _rb = effect_sandbox_rollback(sb1);\n\
         let after_rb = effect_store_get(st, \"{k_alpha}\") + \"|\" + effect_store_get(st, \"{k_beta}\");\n\
         let sb2 = effect_sandbox_begin(st);\n\
         let _s3 = effect_sandbox_delete(sb2, \"{k_alpha}\");\n\
         let _s4 = effect_sandbox_put(sb2, \"{k_beta}\", \"{v_new}\");\n\
         let _cm = effect_sandbox_commit(sb2, st);\n\
         let after_cm = effect_store_get(st, \"{k_alpha}\") + \"|\" + effect_store_get(st, \"{k_beta}\");\n\
         \n\
         let r_get = http_get(\"http://127.0.0.1:{port}{get_path}\");\n\
         let trace_h = http_header_get(r_get, \"x-pilot-trace\");\n\
         let r_post = http_post(\"http://127.0.0.1:{port}{post_path}\", \"application/json\", \"{post_body_esc}\");\n\
         \n\
         let rx_ok = d0 == {exp_d0} && d_same == {exp_d0} && c_noop == 0 && d1 == {exp_d1} && c_delta == 1;\n\
         let ef_ok = iso_before == \"{v_init}\" && sb_view == \"{v_upd}\" && after_rb == \"{v_init}|\" && after_cm == \"|{v_new}\";\n\
         let __res = (if rx_ok {{ \"RX_OK\" }} else {{ \"RX_BAD\" }}) + \"|\"\n\
             + (if ef_ok {{ \"EF_OK\" }} else {{ \"EF_BAD\" }}) + \"|\"\n\
             + trace_h + \"|\" + r_get.body + \"|\" + r_post.body;\n\
         if r_get.status == 200 && r_post.status == 201 {{\n\
             __res\n\
         }} else {{\n\
             \"HTTP_STATUS_BAD\"\n\
         }};\n",
        post_body_esc = escape_pilot_str(&post_req_body),
    );

    let arena = parse(&script);
    assert!(
        arena.diagnostics.is_empty(),
        "reactive/effects/http parse diagnostics: {:?}",
        arena.diagnostics
    );
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    let sockets: Arc<Mutex<Vec<Option<std::net::TcpStream>>>> = Arc::new(Mutex::new(Vec::new()));
    let sock_ref = Arc::clone(&sockets);
    vm.engine_handler = Some(Box::new(move |hook, args| match hook {
        "tcp_connect" => {
            let addr = args.first().map(|v| v.to_string_repr()).unwrap_or_default();
            match pilot_rt::tcp_connect(&addr) {
                Ok(s) => {
                    let mut tbl = sock_ref.lock().unwrap();
                    let fd = tbl.len() as i64;
                    tbl.push(Some(s));
                    Ok(PilotVal::from_int(fd))
                }
                Err(_) => Ok(PilotVal::from_int(-1)),
            }
        }
        "tcp_send" => {
            let fd = args.first().and_then(|v| v.as_int()).unwrap_or(-1);
            let data = args.get(1).map(|v| v.to_string_repr()).unwrap_or_default();
            let mut tbl = sock_ref.lock().unwrap();
            if fd >= 0 && (fd as usize) < tbl.len() {
                if let Some(ref mut stream) = tbl[fd as usize] {
                    stream.write_all(data.as_bytes()).map_err(|e| e.to_string())?;
                    let _ = stream.flush();
                    return Ok(PilotVal::from_int(data.len() as i64));
                }
            }
            Ok(PilotVal::from_int(-1))
        }
        "tcp_recv" => {
            let fd = args.first().and_then(|v| v.as_int()).unwrap_or(-1);
            let max_len = args.get(1).and_then(|v| v.as_int()).unwrap_or(65536).max(1) as usize;
            let mut tbl = sock_ref.lock().unwrap();
            if fd >= 0 && (fd as usize) < tbl.len() {
                if let Some(ref mut stream) = tbl[fd as usize] {
                    let mut buf = vec![0u8; max_len.min(65536)];
                    let mut out = Vec::new();
                    loop {
                        match stream.read(&mut buf) {
                            Ok(0) => break,
                            Ok(n) => {
                                out.extend_from_slice(&buf[..n]);
                                if out.len() >= max_len {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    return Ok(PilotVal::from_str(&String::from_utf8_lossy(&out)));
                }
            }
            Ok(PilotVal::from_str(""))
        }
        "tcp_close" => {
            let fd = args.first().and_then(|v| v.as_int()).unwrap_or(-1);
            let mut tbl = sock_ref.lock().unwrap();
            if fd >= 0 && (fd as usize) < tbl.len() {
                tbl[fd as usize] = None;
            }
            Ok(PilotVal::from_int(0))
        }
        _ => Err(format!("Unhandled hook: {}", hook)),
    }));
    vm.load_binary(bin);
    let out = vm.run().to_string_repr();
    server_thread.join().expect("HTTP loopback server thread panicked");

    let parts: Vec<&str> = out.splitn(5, '|').collect();
    assert_eq!(parts.len(), 5, "Unexpected output: {}", out);
    assert_eq!(parts[0], "RX_OK", "Diamond reactive glitch-free / idempotence test failed");
    assert_eq!(parts[1], "EF_OK", "CoW EffectPlan rollback/commit isolation failed");
    assert_eq!(parts[2], expected_trace_hdr, "Case-insensitive HTTP header lookup failed");
    assert_eq!(parts[3], expected_get_body, "HTTP/1.1 chunked body decoding failed");
    assert_eq!(parts[4], post_resp_body, "HTTP/1.1 POST response body mismatch");

    let reqs = captured_requests.lock().unwrap();
    assert_eq!(reqs.len(), 2);
    assert!(
        reqs[0].starts_with(&format!("GET {} HTTP/1.1\r\n", get_path))
            && reqs[0].to_ascii_lowercase().contains(&format!("host: 127.0.0.1:{}\r\n", port)),
        "Invalid HTTP/1.1 GET wire request:\n{}",
        reqs[0]
    );
    assert!(
        reqs[1].starts_with(&format!("POST {} HTTP/1.1\r\n", post_path))
            && reqs[1].ends_with(&post_req_body),
        "Invalid HTTP/1.1 POST wire request:\n{}",
        reqs[1]
    );
    guard.finish();
}

#[test]
pub fn test_58_4a_shm_bounds_json_marshal_and_polyglot_python_node_bridges() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_4a_shm_bounds_json_marshal_and_polyglot_python_node_bridges",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let shm_src = std::fs::read_to_string(root.join("stdlib/polyglot/shm.pilot")).unwrap();
    let marshal_src = std::fs::read_to_string(root.join("stdlib/polyglot/marshal.pilot")).unwrap();
    let bridge_src = std::fs::read_to_string(root.join("stdlib/polyglot/bridge.pilot")).unwrap();
    assert!(shm_src.lines().count() <= 1500);
    assert!(marshal_src.lines().count() <= 1500);
    assert!(bridge_src.lines().count() <= 1500);

    let v_num = rng.range_i64(-9000, -17);
    let v_a = rng.range_i64(15, 250);
    let v_b = rng.range_i64(15, 250);
    let label = format!("node_{}_\"quoted\"", rng.ascii_payload(6));
    let json_input = format!(
        "{{\"meta\":{{\"id\":{},\"label\":\"{}\",\"ok\":true}},\"items\":[{},{}]}}",
        v_num,
        escape_pilot_str(&label),
        v_a,
        v_b
    );
    let bindings_json = format!("{{\"x\":{},\"y\":{}}}", v_a, v_b);

    let script = format!(
        "{shm_src}\n{marshal_src}\n{bridge_src}\n\
         let buf = create_shm_owned_buffer(8192, 128, 16);\n\
         let v_ok = project_shm_borrowed_view(buf, 4, 8, 8);\n\
         let v_oob = project_shm_borrowed_view(buf, 12, 8, 8);\n\
         let doc = json_parse(\"{json_esc}\");\n\
         let j_id = json_get_int(doc, \"meta.id\");\n\
         let j_lbl = json_get_str(doc, \"meta.label\");\n\
         let j_ok = json_get_bool(doc, \"meta.ok\");\n\
         let j_len = json_array_len(doc, \"items\");\n\
         let rt_doc = json_parse(json_stringify(doc));\n\
         let rt_id = json_get_int(rt_doc, \"meta.id\");\n\
         let py_out = polyglot_eval_json(\"python\", \"{bind_esc}\", \"x * y + 7\");\n\
         let js_out = polyglot_eval_json(\"node\", \"{bind_esc}\", \"(x + y) * 3\");\n\
         let shm_pass = buf.lattice_bits == 2 && v_ok.valid && v_ok.base_addr == 8224 && v_ok.byte_len == 64 && v_ok.lattice_bits == 1 && !v_oob.valid && v_oob.byte_len == 0;\n\
         let json_pass = j_id == {v_num} && rt_id == {v_num} && j_lbl == \"{lbl_esc}\" && j_ok && j_len == 2;\n\
         let __res = (if shm_pass {{ \"SHM_OK\" }} else {{ \"SHM_BAD\" }}) + \"|\"\n\
             + (if json_pass {{ \"JSON_OK\" }} else {{ \"JSON_BAD\" }}) + \"|\"\n\
             + py_out + \"|\" + js_out;\n\
         __res;\n",
        json_esc = escape_pilot_str(&json_input),
        bind_esc = escape_pilot_str(&bindings_json),
        lbl_esc = escape_pilot_str(&label),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "polyglot parse diagnostics: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.engine_handler = Some(Box::new(|hook, args| match hook {
        "host_spawn_eval" => {
            let runtime_bin = args.first().map(|v| v.to_string_repr()).unwrap_or_default();
            let code = args.get(1).map(|v| v.to_string_repr()).unwrap_or_default();
            let flag = if runtime_bin.contains("node") { "-e" } else { "-c" };
            let out = Command::new(&runtime_bin)
                .arg(flag)
                .arg(&code)
                .output()
                .map_err(|e| e.to_string())?;
            Ok(PilotVal::from_str(String::from_utf8_lossy(&out.stdout).trim()))
        }
        _ => Err(format!("Unhandled hook: {}", hook)),
    }));
    vm.load_binary(bin);
    let res = vm.run().to_string_repr();
    let cols: Vec<&str> = res.split('|').collect();
    assert_eq!(cols.len(), 4, "Unexpected polyglot output: {}", res);
    assert_eq!(cols[0], "SHM_OK", "SHM bounds/lattice projection failed");
    assert_eq!(cols[1], "JSON_OK", "JSON parse/stringify round-trip failed");
    assert_eq!(
        cols[2].trim(),
        (v_a * v_b + 7).to_string(),
        "Python polyglot bridge evaluation mismatch"
    );
    assert_eq!(
        cols[3].trim(),
        ((v_a + v_b) * 3).to_string(),
        "Node polyglot bridge evaluation mismatch"
    );
    guard.finish();
}

#[test]
pub fn test_58_4b_relational_sql_sqlite3_oracle_and_postgres_mysql_binary_wire_frames() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_4b_relational_sql_sqlite3_oracle_and_postgres_mysql_binary_wire_frames",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    let mut rng = SplitMix64::from_entropy();

    let sql_src = std::fs::read_to_string(root.join("stdlib/db/sql_engine.pilot")).unwrap();
    let pg_src = std::fs::read_to_string(root.join("stdlib/db/postgres_wire.pilot")).unwrap();
    let my_src = std::fs::read_to_string(root.join("stdlib/db/mysql_wire.pilot")).unwrap();
    assert!(sql_src.lines().count() <= 1500);
    assert!(pg_src.lines().count() <= 1500);
    assert!(my_src.lines().count() <= 1500);
    assert!(
        !sql_src.contains("item_"),
        "sql_engine.pilot must not contain legacy fabricated 'item_' mock rows"
    );

    // Build randomized relational dataset and verify every query against Python `sqlite3`!
    let d1 = format!("Core_{}", rng.ascii_payload(5));
    let d2 = format!("Infra_{}", rng.ascii_payload(5));
    let e1 = format!("Ada_{}", rng.ascii_payload(4));
    let e2 = format!("Lin_{}", rng.ascii_payload(4));
    let e3 = format!("Ken_{}", rng.ascii_payload(4));
    let e4 = format!("Grace_{}", rng.ascii_payload(4));
    let s1 = rng.range_i64(110, 145);
    let s2 = rng.range_i64(150, 185);
    let s3 = rng.range_i64(80, 105);
    let s4 = rng.range_i64(190, 240);
    let bonus = rng.range_i64(15, 35);

    let statements = vec![
        "CREATE TABLE dept (id INT, title TEXT);".to_string(),
        "CREATE TABLE emp (id INT, name TEXT, dept_id INT, salary INT, active INT);".to_string(),
        format!("INSERT INTO dept VALUES (10, '{}');", d1),
        format!("INSERT INTO dept VALUES (20, '{}');", d2),
        format!("INSERT INTO emp VALUES (1, '{}', 10, {}, 1);", e1, s1),
        format!("INSERT INTO emp VALUES (2, '{}', 20, {}, 1);", e2, s2),
        format!("INSERT INTO emp VALUES (3, '{}', 10, {}, 0);", e3, s3),
        format!("INSERT INTO emp VALUES (4, '{}', 20, {}, 1);", e4, s4),
        "SELECT name, salary FROM emp WHERE active = 1 AND salary >= 145 ORDER BY salary DESC;".to_string(),
        "SELECT emp.name, dept.title FROM emp JOIN dept ON emp.dept_id = dept.id WHERE emp.active = 1 ORDER BY emp.id ASC;".to_string(),
        format!("UPDATE emp SET salary = {} WHERE id = 1;", s1 + bonus),
        "DELETE FROM emp WHERE active = 0;".to_string(),
        "SELECT id, name, salary FROM emp ORDER BY id ASC;".to_string(),
    ];

    // Run exact SQL workload in Python `sqlite3` `:memory:` oracle
    let py_sqlite = r#"
import json, sqlite3, sys
stmts = json.loads(sys.stdin.read())
con = sqlite3.connect(":memory:")
cur = con.cursor()
select_outputs = []
for s in stmts:
    cur.execute(s)
    if s.lstrip().upper().startswith("SELECT"):
        rows = ["|".join(str(c) for c in r) for r in cur.fetchall()]
        select_outputs.append("\n".join(rows))
print(json.dumps(select_outputs))
"#;
    let stmts_json = format!(
        "[{}]",
        statements
            .iter()
            .map(|s| format!("\"{}\"", escape_pilot_str(s)))
            .collect::<Vec<_>>()
            .join(",")
    );
    let mut py = Command::new("python3")
        .arg("-c")
        .arg(py_sqlite)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    py.stdin.as_mut().unwrap().write_all(stmts_json.as_bytes()).unwrap();
    let py_out = py.wait_with_output().unwrap();
    assert!(py_out.status.success());

    // Parse the 3 expected SELECT results via python helper or direct comparison
    let py_sel_lines = {
        let mut p2 = Command::new("python3")
            .arg("-c")
            .arg("import json,sys; arr=json.loads(sys.stdin.read()); print('===SEL==='.join(arr))")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        p2.stdin.as_mut().unwrap().write_all(&py_out.stdout).unwrap();
        String::from_utf8(p2.wait_with_output().unwrap().stdout).unwrap().trim().to_string()
    };

    // Binary wire protocol payloads > 260 bytes so multi-byte BE u32 and 3-byte LE lengths are tested!
    let long_sql = format!("SELECT payload FROM audit_log WHERE token = '{}';", rng.ascii_payload(275));
    let pg_user = format!("pilot_{}", rng.ascii_payload(7));
    let pg_db = format!("warehouse_{}", rng.ascii_payload(7));
    let dr_c0 = rng.ascii_payload(18);
    let dr_c1 = rng.ascii_payload(265);
    let my_seq = rng.range_i64(3, 250);

    // Reference PostgreSQL v3 Query frame hex: 'Q' (0x51) + BE u32 (4 + sql.len() + 1) + sql + 0x00
    let mut exp_pg_q = Vec::with_capacity(1 + 4 + long_sql.len() + 1);
    exp_pg_q.push(b'Q');
    exp_pg_q.extend_from_slice(&((4 + long_sql.len() + 1) as u32).to_be_bytes());
    exp_pg_q.extend_from_slice(long_sql.as_bytes());
    exp_pg_q.push(0);
    let exp_pg_q_hex: String = exp_pg_q.iter().map(|b| format!("{:02x}", b)).collect();

    // Reference MySQL COM_QUERY frame hex: 3-byte LE (1 + sql.len()) + seq_id + 0x03 + sql
    let my_len = (1 + long_sql.len()) as u32;
    let mut exp_my_q = Vec::with_capacity(4 + 1 + long_sql.len());
    exp_my_q.push((my_len & 0xFF) as u8);
    exp_my_q.push(((my_len >> 8) & 0xFF) as u8);
    exp_my_q.push(((my_len >> 16) & 0xFF) as u8);
    exp_my_q.push(my_seq as u8);
    exp_my_q.push(0x03);
    exp_my_q.extend_from_slice(long_sql.as_bytes());
    let exp_my_q_hex: String = exp_my_q.iter().map(|b| format!("{:02x}", b)).collect();

    let mut pilot_sql_block = String::from("let db = sql_db_new();\nvar sel_joined = \"\";\nvar first_sel = true;\n");
    for (idx, s) in statements.iter().enumerate() {
        let esc = escape_pilot_str(s);
        if s.starts_with("SELECT") {
            pilot_sql_block.push_str(&format!(
                "let r_{idx} = sql_exec(db, \"{esc}\");\n\
                 if !first_sel {{ sel_joined = sel_joined + \"===SEL===\"; }}\n\
                 first_sel = false;\n\
                 sel_joined = sel_joined + r_{idx};\n"
            ));
        } else {
            pilot_sql_block.push_str(&format!("let _e_{idx} = sql_exec(db, \"{esc}\");\n"));
        }
    }

    let script = format!(
        "{sql_src}\n{pg_src}\n{my_src}\n{pilot_sql_block}\n\
         let pg_st = pg_encode_startup_hex(\"{pg_user}\", \"{pg_db}\");\n\
         let pg_st_dec = pg_decode_startup_info(pg_st);\n\
         let pg_q = pg_encode_query_hex(\"{long_sql_esc}\");\n\
         let pg_q_dec = pg_decode_query_sql(pg_q);\n\
         let pg_dr = pg_encode_data_row_hex(\"{dr_c0}\", \"{dr_c1}\");\n\
         let pg_dr_dec = pg_decode_data_row_pipe(pg_dr);\n\
         let my_q = mysql_encode_com_query_hex({my_seq}, \"{long_sql_esc}\");\n\
         let my_q_dec = mysql_decode_com_query(my_q);\n\
         let __out = sel_joined + \"@@WIRE@@\" + pg_st_dec + \"@@WIRE@@\" + pg_q + \"@@WIRE@@\" + pg_q_dec\n\
             + \"@@WIRE@@\" + pg_dr_dec + \"@@WIRE@@\" + my_q + \"@@WIRE@@\" + my_q_dec;\n\
         __out;\n",
        long_sql_esc = escape_pilot_str(&long_sql),
    );

    let arena = parse(&script);
    assert!(arena.diagnostics.is_empty(), "db/wire parse diagnostics: {:?}", arena.diagnostics);
    let bin = BytecodeCompiler::compile(&arena, arena.root);
    let mut vm = PilVirtualMachine::new();
    vm.load_binary(bin);
    let out = vm.run().to_string_repr();
    let segs: Vec<&str> = out.split("@@WIRE@@").collect();
    assert_eq!(segs.len(), 7, "Unexpected wire/SQL output: {}", out);

    assert_eq!(segs[0], py_sel_lines, "Embedded SQL relational engine output diverged from sqlite3!");
    assert_eq!(segs[1], format!("{}|{}", pg_user, pg_db), "PostgreSQL v3 StartupMessage decode mismatch");
    assert_eq!(segs[2], exp_pg_q_hex, "PostgreSQL v3 Simple Query >256B binary frame mismatch");
    assert_eq!(segs[3], long_sql, "PostgreSQL v3 Simple Query decode mismatch");
    assert_eq!(segs[4], format!("{}|{}", dr_c0, dr_c1), "PostgreSQL v3 DataRow >256B decode mismatch");
    assert_eq!(segs[5], exp_my_q_hex, "MySQL COM_QUERY 3-byte LE binary frame mismatch");
    assert_eq!(segs[6], format!("{}|{}", my_seq, long_sql), "MySQL COM_QUERY decode mismatch");
    guard.finish();
}

#[test]
pub fn test_58_5_legacy_polyglot_crypto_deleted_and_strict_rust_loc_caps() {
    let guard = anticheat::AntiCheatGuard::arm(
        "test_58_5_legacy_polyglot_crypto_deleted_and_strict_rust_loc_caps",
        &ANTICHEAT_SPEC,
    );
    let root = workspace_root();
    assert!(
        !root.join("crates/pilot-polyglot").exists(),
        "crates/pilot-polyglot must be completely deleted from disk in Milestone 58.5"
    );
    assert!(
        !root.join("crates/pilot-rt/src/crypto.rs").exists(),
        "crates/pilot-rt/src/crypto.rs must be completely deleted from disk in Milestone 58.5"
    );

    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        !cargo_toml.contains("pilot-polyglot"),
        "Workspace Cargo.toml must not reference retired pilot-polyglot"
    );

    let mut rt_lines = 0usize;
    for entry in std::fs::read_dir(root.join("crates/pilot-rt/src")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            rt_lines += content.lines().count();
            let non_comment_code: String = content
                .lines()
                .map(|l| l.split("//").next().unwrap_or(""))
                .collect::<Vec<_>>()
                .join("\n");
            for banned in ["blake3", "ed25519", "SliceBuffer", "EffectPlan", "HTTP/1.1"] {
                assert!(
                    !non_comment_code.contains(banned),
                    "crates/pilot-rt/src/{} contains banned legacy symbol '{}'",
                    path.file_name().unwrap().to_string_lossy(),
                    banned
                );
            }
        }
    }
    assert!(rt_lines <= 700, "crates/pilot-rt/src total LOC ({}) must be <= 700", rt_lines);

    let mut ffi_lines = 0usize;
    for entry in std::fs::read_dir(root.join("crates/pilot-ffi/src")).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            ffi_lines += std::fs::read_to_string(&path).unwrap().lines().count();
        }
    }
    assert!(ffi_lines <= 300, "crates/pilot-ffi/src total LOC ({}) must be <= 300", ffi_lines);
    guard.finish();
}
