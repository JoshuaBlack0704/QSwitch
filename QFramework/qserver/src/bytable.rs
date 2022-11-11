use super::{Bytable, terminal_map::MessageExchangeHeader};

impl Bytable for MessageExchangeHeader{
    fn to_bytes(&self, dst: &[u8]) {
        let ptr = self as *const Self;
        let bytes = unsafe{std::slice::from_raw_parts(ptr as *const u8, size_of::<Self>())};
        assert(bytes.len() <= dst.len());
        
         
    }

    fn from_bytes(src: &[u8]) -> Self {
        todo!()
    }
}