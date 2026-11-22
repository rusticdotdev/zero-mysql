use crate::BufferSet;

#[test]
fn buffer_set_new() {
    let buffers = BufferSet::new();
    assert!(buffers.initial_handshake.is_empty());
    assert!(buffers.read_buffer.is_empty());
    assert_eq!(buffers.write_buffer().len(), 4); // pre-allocated header space
}

#[test]
fn buffer_set_default() {
    let buffers = BufferSet::default();
    assert!(buffers.initial_handshake.is_empty());
}

#[test]
fn buffer_set_with_initial_handshake() {
    let handshake = vec![0x0a, 0x35, 0x2e, 0x37];
    let buffers = BufferSet::with_initial_handshake(handshake.clone());
    assert_eq!(buffers.initial_handshake, handshake);
    assert!(buffers.read_buffer.is_empty());
}

#[test]
fn new_write_buffer() {
    let mut buffers = BufferSet::new();
    let buf = buffers.new_write_buffer();
    // Should have 4 bytes reserved for header
    assert_eq!(buf.len(), 4);
    assert_eq!(buffers.payload_len(), 0);
}

#[test]
fn write_buffer_mut() {
    let mut buffers = BufferSet::new();
    buffers.new_write_buffer().extend_from_slice(b"SELECT 1");
    // 4 header bytes + 8 payload bytes
    assert_eq!(buffers.write_buffer().len(), 12);
    assert_eq!(buffers.payload_len(), 8);
}

#[test]
fn write_buffer() {
    let mut buffers = BufferSet::new();
    buffers.new_write_buffer().extend_from_slice(b"test");
    let packet = buffers.write_buffer();
    assert_eq!(packet.len(), 8); // 4 header + 4 payload
    assert_eq!(&packet[4..], b"test");
}

#[test]
fn buffer_reuse() {
    let mut buffers = BufferSet::new();

    // Write to buffers
    buffers.read_buffer.extend_from_slice(b"test data");
    buffers.new_write_buffer().extend_from_slice(b"query");

    assert_eq!(buffers.read_buffer.len(), 9);
    assert_eq!(buffers.payload_len(), 5);

    // Clear and reuse
    buffers.read_buffer.clear();
    buffers.new_write_buffer();

    assert_eq!(buffers.read_buffer.len(), 0);
    assert_eq!(buffers.payload_len(), 0);

    // Capacity should be preserved
    assert!(buffers.read_buffer.capacity() >= 9);
}

#[test]
fn return_buffer_set_shrinks_to_soft_buffer_cap() {
    use std::sync::Arc;

    use crate::BufferPool;

    const SOFT_BUFFER_CAP: usize = 1024;
    const GROWN: usize = 1024 * 1024;

    let mut pool = BufferPool::new(1);
    pool.soft_buffer_cap = Some(SOFT_BUFFER_CAP);
    let pool = Arc::new(pool);

    let mut buffer_set = pool.get_buffer_set();
    buffer_set.read_buffer.resize(GROWN, 0);
    buffer_set.new_write_buffer().resize(GROWN, 0);
    drop(buffer_set);

    let recycled = pool.get_buffer_set();
    assert!(recycled.read_buffer.is_empty());
    assert!(recycled.read_buffer.capacity() < GROWN);
    assert_eq!(recycled.write_buffer().len(), 4);
    assert!(recycled.write_buffer.capacity() < GROWN);
}
