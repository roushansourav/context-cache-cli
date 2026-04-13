/// Compute a fast BLAKE3 hash over a byte slice; return hex string.
pub fn hash_bytes(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

/// Cheap file-change probe: compare mtime + size before re-reading content.
pub fn file_changed(
    mtime_ms: i64,
    size: u64,
    prev_mtime_ms: i64,
    prev_size: u64,
) -> bool {
    mtime_ms != prev_mtime_ms || size != prev_size
}
