//! Milestone 34: Zero-GC Native Runtime (`pilot-rt`).
//!
//! Enforces:
//! - 34.1: 16-Byte Unboxed C-ABI Value Representation (`PilotVal`) with SSO up to 14 bytes.
//! - 34.2: 3-Tier Deterministic Memory Engine (64KB Bump Region + Perceus ARC Header).
//! - 34.4: Native Concurrency, MPMC Channels, Signals, and Shadow Effect Dry-Runs.

pub mod value;
pub mod memory;
pub mod concurrency;

pub use value::{PilotVal, TAG_INT, TAG_FLOAT, TAG_BOOL, TAG_SSO_STR, TAG_NULL};
pub use memory::{ArcHeader, BumpRegion, FLAG_ATOMIC};
pub use concurrency::{Channel, ShadowEffectContext, Signal};

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
}
