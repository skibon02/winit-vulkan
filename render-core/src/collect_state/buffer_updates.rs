use std::ops::Range;

#[derive(Debug)]
pub struct BufferUpdateData<'a> {
    pub modified_bytes: &'a [u8],
    pub buffer_offset: usize
}

pub enum BufferUpdateCmd<'a> {
    /// 0: new buffer length
    Update(BufferUpdateData<'a>),
    Resize(usize),
    Rearrange(Vec<(Range<usize>, usize)>),
}