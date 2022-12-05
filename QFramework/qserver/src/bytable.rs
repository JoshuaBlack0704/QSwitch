use std::mem::size_of;

use crate::{Bytable, message_exchange::MessageExchangeHeader, station::StationHeader};


impl Bytable for MessageExchangeHeader{
    fn to_bytes(&self, dst: &mut [u8]) {
        let ptr = self as *const Self;
        let bytes = unsafe{std::slice::from_raw_parts(ptr as *const u8, size_of::<Self>())};
        assert!(bytes.len() <= dst.len());
        
        for (index, byte) in bytes.iter().enumerate(){
            dst[index] = *byte;
        }
        
         
    }

    fn from_bytes(src: &[u8]) -> Self {
        let data = src.as_ptr();
        unsafe{std::slice::from_raw_parts(data as *const Self, 1)[0].clone()}
    }
}

impl Into<Vec<u8>> for MessageExchangeHeader{
    fn into(self) -> Vec<u8> {
        let mut data = vec![0; size_of::<Self>()];
        self.to_bytes(&mut data);
        data
    }
}
impl Into<[u8; size_of::<Self>()]> for MessageExchangeHeader{
    fn into(self) -> [u8; size_of::<Self>()] {
        let mut data = [0u8; size_of::<Self>()];
        self.to_bytes(&mut data);
        data
    }
}

impl<T: Clone> Bytable for Vec<T>{
    fn to_bytes(&self, dst: &mut [u8]) {
        let ptr = self.as_ptr();
        let bytes = unsafe{std::slice::from_raw_parts(ptr as *const u8, size_of::<T>() * self.len())};
        assert!(bytes.len() <= dst.len());
        
        for (index, byte) in bytes.iter().enumerate(){
            dst[index] = *byte;
        }
    }

    fn from_bytes(src: &[u8]) -> Self {
        //We need to make sure are slice will return whole structs
        assert!(src.len()%size_of::<T>() == 0);
        let count = src.len()/size_of::<T>();
        let data = src.as_ptr();
        unsafe{std::slice::from_raw_parts(data as *const T, count).to_vec()}
    }
}

// impl<T:Clone> Into<Vec<u8>> for Vec<T>{
//     fn into(self) -> Vec<u8> {
//         let mut data = vec![0; size_of::<Self>()];
//         self.to_bytes(&mut data);
//         data
//     }
// }


impl Bytable for StationHeader{
    fn to_bytes(&self, dst: &mut [u8]) {
        let ptr = self as *const Self;
        let bytes = unsafe{std::slice::from_raw_parts(ptr as *const u8, size_of::<Self>())};
        assert!(bytes.len() <= dst.len());
        
        for (index, byte) in bytes.iter().enumerate(){
            dst[index] = *byte;
        }
        
         
    }

    fn from_bytes(src: &[u8]) -> Self {
        let data = src.as_ptr();
        unsafe{std::slice::from_raw_parts(data as *const Self, 1)[0].clone()}
    }
}
impl Into<Vec<u8>> for StationHeader{
    fn into(self) -> Vec<u8> {
        let mut data = vec![0; size_of::<Self>()];
        self.to_bytes(&mut data);
        data
    }
}
impl Into<[u8; size_of::<Self>()]> for StationHeader{
    fn into(self) -> [u8; size_of::<Self>()] {
        let mut data = [0u8; size_of::<Self>()];
        self.to_bytes(&mut data);
        data
    }
}
