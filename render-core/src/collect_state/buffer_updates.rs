

#[derive(Debug)]
pub struct StaticBufferUpdates<'a> {
    pub modified_bytes: &'a [u8],
    pub buffer_offset: usize
}