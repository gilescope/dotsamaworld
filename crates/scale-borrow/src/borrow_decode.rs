use parity_scale_codec::{Compact, Decode};

pub trait BorrowDecode<'scale> {
    fn borrow_decode(data: &'scale [u8]) -> Self;
}

impl<'scale> BorrowDecode<'scale> for &'scale str {
    fn borrow_decode(data: &'scale [u8]) -> Self {
        std::str::from_utf8(data).unwrap()
    }
}

impl<'scale> BorrowDecode<'scale> for &'scale [u8] {
    fn borrow_decode(data: &'scale [u8]) -> Self {
        data
    }
}

macro_rules! impl_borrow_decode {
    ($($t:ty)+) => {
        $(
            impl<'scale> BorrowDecode<'scale> for $t {
                fn borrow_decode(mut data: &'scale [u8]) -> Self {
                    let d = &mut data;
                    <$t>::decode(d).unwrap()
                }
            }
        )+
    };
}

// Just delegate to standard scale decode
impl_borrow_decode!(bool u8 u16 u32 u64 u128 Compact<u32> Compact<u64> Compact<u128>);
