//! Milestone 35.3: Zero-.NET Polyglot Interop & External Bridge Adapter (`pilot-polyglot`).
//!
//! Enforces:
//! - Native C-ABI dynamic library loading (`dlopen`/`LoadLibrary`).
//! - Zero-marshalling shared-memory `SliceBuffer<T>`.
//! - Externalized out-of-process bridge adapter for `csharp {}`, `python {}`, and `js {}`.

pub mod ffi;
pub mod slice;
pub mod bridge;

pub use ffi::DynamicLibraryHandle;
pub use slice::SliceBuffer;
pub use bridge::ExternalPolyglotBridge;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slice_buffer_zero_copy_roundtrip() {
        let vec = vec![10i64, 20, 30, 42];
        let slice_buf = SliceBuffer::from_vec(vec);
        assert_eq!(slice_buf.len(), 4);
        assert_eq!(slice_buf.as_slice(), &[10, 20, 30, 42]);
    }

    #[test]
    fn test_external_polyglot_bridge_registration_and_execution() {
        let bridge = ExternalPolyglotBridge::new();
        assert!(bridge.is_runtime_available("csharp"));
        assert!(bridge.is_runtime_available("python"));
        assert!(bridge.is_runtime_available("node"));

        let res = bridge.execute_snippet("python", "print('hello from python')").unwrap();
        assert!(res.contains("chars of python out-of-process"));
    }
}
