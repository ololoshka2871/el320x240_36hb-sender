use std::time::Duration;

use bytes::{BufMut, BytesMut};

pub(crate) fn display_sender(
    port: String,
    rx: futures_intrusive::channel::shared::GenericReceiver<
        parking_lot::RawMutex,
        Vec<u8>,
        futures_intrusive::buffer::GrowingHeapBuf<Vec<u8>>,
    >,
) {
    const BLOCK_SIZE: usize = 64;
    const BLOCK_SIZE_PYLOAD: usize = BLOCK_SIZE - std::mem::size_of::<u32>();

    let mut port = serialport::new(port, 15000000)
        .timeout(Duration::from_millis(5))
        .open()
        .expect("Failed to open port");

    pollster::block_on(async {
        loop {
            if let Some(frame) = rx.receive().await {
                frame
                    .chunks(BLOCK_SIZE_PYLOAD)
                    .enumerate()
                    .for_each(|(i, data)| {
                        let offset = i * BLOCK_SIZE_PYLOAD;
                        let mut buf: BytesMut = BytesMut::new();
                        // offset in bytes
                        buf.put_u32_le(offset as u32);
                        buf.put_slice(data);

                        port.write_all(&buf).unwrap();
                    });
            }
        }
    });
}
