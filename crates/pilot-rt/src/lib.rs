//! Milestone 34: Zero-GC Native Runtime (`pilot-rt`).
//!
//! Enforces:
//! - 34.1: 16-Byte Unboxed C-ABI Value Representation (`PilotVal`) with SSO up to 14 bytes.
//! - 34.2: 3-Tier Deterministic Memory Engine (64KB Bump Region + Perceus ARC Header).
//! - 34.4: Native Concurrency, MPMC Channels, Signals, and Shadow Effect Dry-Runs.

pub mod value;
pub mod memory;
pub mod concurrency;
pub mod stdlib;
pub mod c_abi;
pub mod crypto;
pub mod time;
pub mod net;

pub use value::{PilotVal, TAG_INT, TAG_FLOAT, TAG_BOOL, TAG_SSO_STR, TAG_NULL};
pub use memory::{ArcHeader, BumpRegion, FLAG_ATOMIC};
pub use concurrency::{Channel, ShadowEffectContext, Signal};
pub use stdlib::{PilotDfaRegex, PilotStdlibIntrinsics, PilotStdlibResolver};
pub use crypto::{blake3, ed25519_sign, ed25519_verify, hmac_sha256, random_bytes, sha256, sha512};
pub use time::{monotonic_micros, now_millis, now_nanos, sleep_ms, timespec_now};
pub use net::{http_get, http_post, tcp_connect, tcp_listen, udp_bind, HttpResponse};
pub use c_abi::{
    generate_c_runtime_header, pilot_rt_alloc_arc, pilot_rt_crypto_random_bytes,
    pilot_rt_crypto_sha256, pilot_rt_drop, pilot_rt_drop_reuse, pilot_rt_dup, pilot_rt_print_f64,
    pilot_rt_print_i64, pilot_rt_region_alloc, pilot_rt_region_enter, pilot_rt_region_exit,
    pilot_rt_sleep_ms, pilot_rt_time_now_ns,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pilot_val_unboxed_layout_and_arithmetic() {
        assert_eq!(core::mem::size_of::<PilotVal>(), 16);
        assert_eq!(core::mem::align_of::<PilotVal>(), 8);

        let a = PilotVal::from_int(40);
        let b = PilotVal::from_int(2);
        let c = a.add(b).unwrap();
        assert_eq!(c.as_int(), Some(42));

        let fa = PilotVal::from_float(3.5);
        let fb = PilotVal::from_float(1.5);
        let fc = fa.add(fb).unwrap();
        assert_eq!(fc.as_float(), Some(5.0));
    }

    #[test]
    fn test_small_string_optimization_up_to_14_bytes_zero_alloc() {
        // String with 11 bytes: "Hello World"
        let s = "Hello World";
        let val = PilotVal::from_str_sso(s).expect("11 bytes must fit in 14-byte SSO");
        assert!(val.is_sso_str());

        val.with_str(|extracted| {
            assert_eq!(extracted, "Hello World");
        });

        // String with exact boundary of 14 bytes: "12345678901234"
        let s14 = "12345678901234";
        let val14 = PilotVal::from_str_sso(s14).expect("14 bytes must fit in 14-byte SSO");
        assert!(val14.is_sso_str());
        val14.with_str(|extracted| {
            assert_eq!(extracted, "12345678901234");
        });

        // 15 bytes exceeds SSO -> None
        let s15 = "123456789012345";
        assert!(PilotVal::from_str_sso(s15).is_none());
    }

    #[test]
    fn test_3tier_memory_engine_region_reset_and_perceus_arc() {
        // Tier 1: 64KB Bump Region
        let mut region = BumpRegion::new();
        let wm0 = region.watermark();
        let p1 = region.alloc(128, 8).unwrap();
        assert!(!p1.is_null());
        assert!(region.watermark() > wm0);

        // O(1) watermark reset
        region.reset_watermark(wm0);
        assert_eq!(region.watermark(), wm0);

        // Tier 2: Perceus ARC Header
        let mut arc = ArcHeader::new(1, false);
        assert_eq!(arc.rc, 1);
        assert!(arc.can_drop_reuse());

        arc.retain();
        assert_eq!(arc.rc, 2);
        assert!(!arc.can_drop_reuse());

        let destroyed = arc.release();
        assert!(!destroyed);
        assert_eq!(arc.rc, 1);
        assert!(arc.can_drop_reuse());

        let destroyed_final = arc.release();
        assert!(destroyed_final);
    }

    #[test]
    fn test_concurrency_mpmc_channel_and_shadow_context() {
        // MPMC Channel
        let chan = Channel::new(10);
        assert!(chan.send(PilotVal::from_int(100)));
        let val = chan.recv().unwrap();
        assert_eq!(val.as_int(), Some(100));

        // Shadow CoW Context
        let mut ctx = ShadowEffectContext::new();
        ctx.write("balance", PilotVal::from_int(500));
        assert_eq!(ctx.read("balance").unwrap().as_int(), Some(500));

        // Rollback
        ctx.rollback();
        assert!(ctx.read("balance").is_none());

        // Commit
        ctx.write("account_id", PilotVal::from_int(42));
        ctx.commit();
        assert!(ctx.is_committed());
        assert_eq!(ctx.read("account_id").unwrap().as_int(), Some(42));
    }

    #[test]
    fn test_stdlib_intrinsics_and_dfa_regex() {
        // Intrinsics
        assert_eq!(PilotStdlibIntrinsics::type_name(PilotVal::from_int(42)), "int");
        assert_eq!(PilotStdlibIntrinsics::type_name(PilotVal::from_float(3.14)), "float");
        assert_eq!(PilotStdlibIntrinsics::math_sqrt(16.0), 4.0);

        // DFA Regex
        let regex = PilotDfaRegex::compile("^hello.*world$");
        assert!(regex.is_match("hello world"));
        assert!(regex.is_match("hello beautiful world"));
        assert!(!regex.is_match("bye world"));

        // Module resolution
        assert!(PilotStdlibResolver::is_stdlib_module("std.math"));
        assert!(PilotStdlibResolver::is_stdlib_module("std.core"));
        assert!(!PilotStdlibResolver::is_stdlib_module("unknown.module"));
    }

    #[test]
    fn test_c_abi_regions_arc_and_drop_reuse() {
        unsafe {
            // Region alloc
            let region = pilot_rt_region_enter();
            let ptr1 = pilot_rt_region_alloc(region, 64, 8);
            assert!(!ptr1.is_null());
            pilot_rt_region_exit(region);

            // ARC alloc & dup
            let arc_ptr = pilot_rt_alloc_arc(32);
            assert!(!arc_ptr.is_null());
            let dup_ptr = pilot_rt_dup(arc_ptr);
            assert_eq!(arc_ptr, dup_ptr);

            // Drop 1 reference
            pilot_rt_drop(arc_ptr, None);

            // Drop-reuse on uniquely owned pointer (recycles without allocation)
            let reused_ptr = pilot_rt_drop_reuse(arc_ptr, 32);
            assert_eq!(arc_ptr, reused_ptr);

            // Final drop
            pilot_rt_drop(reused_ptr, None);

            // Header check
            let header = generate_c_runtime_header();
            assert!(header.contains("pilot_rt_region_enter"));
            assert!(header.contains("pilot_rt_drop_reuse"));
        }
    }

    #[test]
    fn test_crypto_sha256_blake3_and_random() {
        let empty_hash = sha256(b"");
        // Standard SHA-256 of empty string: e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        let hex = empty_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");

        let b3 = blake3(b"hello world");
        assert_eq!(b3.len(), 32);

        let hmac = hmac_sha256(b"secret", b"message");
        assert_eq!(hmac.len(), 32);

        let rand = random_bytes(16);
        assert_eq!(rand.len(), 16);
    }

    #[test]
    fn test_time_and_http_response() {
        let t_ns = now_nanos();
        assert!(t_ns > 0);

        let t_ms = now_millis();
        assert!(t_ms > 0);

        let (sec, nsec) = timespec_now();
        assert!(sec > 0);
        assert!(nsec < 1_000_000_000);

        let resp = HttpResponse::ok(b"{\"status\":\"ok\"}", "application/json");
        assert_eq!(resp.status, 200);
        assert_eq!(resp.text(), "{\"status\":\"ok\"}");
    }
}
