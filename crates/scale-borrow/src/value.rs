use parity_scale_codec::Compact;
use scale_info::PortableRegistry;
use scale_info::TypeDef;
use scale_info::TypeDefPrimitive;

#[cfg(feature = "display")]
use core::fmt::{Display, Formatter};

/// The underlying shape of a given value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value<'scale> {
    /// A named or unnamed struct-like, array-like or tuple-like set of values.
    Object(Box<Vec<(&'scale str, Value<'scale>)>>), // Could this be an array rather than a vec?
    // // UnamedComposite(&'scale Vec<Value<T>>)
    // /// An enum variant.
    // Variant(&'scale (&'scale str, &'scale Value<'scale>)),
    // Truth
    Bool(bool),
    Char(char),
    Str(&'scale str),
    Scale(&'scale [u8]),
    // Escape hatch for when you can't borrow.
    ScaleOwned(Box<Vec<u8>>),
    // Array(Box<Vec<Value<'scale>>>),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(Box<u128>),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(Box<i128>),
    /// An unsigned 256 bit number (internally represented as a 32 byte array).
    U256(&'scale [u8; 32]),
    /// A signed 256 bit number (internally represented as a 32 byte array).
    I256(&'scale [u8; 32]),

    #[cfg(feature = "bitvec")]
    Bits(Box<scale_value::BitSequence>),
}

#[cfg(feature = "display")]
impl<'scale> Display for Value<'scale> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        const TRUNC_LEN: usize = 100;
        match self {
            Self::Object(contents) => {
                write!(f, "{{").unwrap();
                let mut first = true;
                for (k, v) in contents.iter() {
                    if !first {
                        write!(f, ", ").unwrap();
                    }
                    k.fmt(f).unwrap();
                    write!(f, ": ").unwrap();
                    v.fmt(f).unwrap();
                    first = false;
                }
                write!(f, "}}").unwrap();
            }
            Self::Scale(slice) => {
                if slice.len() <= TRUNC_LEN {
                    write!(f, "Scale(0x{})", hex::encode(slice)).unwrap();
                } else {
                    write!(f, "Scale(0x{}...)", hex::encode(&slice[..TRUNC_LEN - 3])).unwrap();
                }
            }
            Self::ScaleOwned(v) => {
                if v.len() <= TRUNC_LEN {
                    write!(f, "ScaleOwned(0x{})", hex::encode(v.as_slice())).unwrap();
                } else {
                    write!(
                        f,
                        "ScaleOwned(0x{}...)",
                        hex::encode(&v.as_slice()[..TRUNC_LEN - 3])
                    )
                    .unwrap();
                }
            }
            _ => <Self as core::fmt::Debug>::fmt(self, f).unwrap(),
        };
        Ok(())
    }
}

impl<'a, 'scale> IntoIterator for &'a Value<'scale> {
    type Item = &'a (&'scale str, Value<'scale>);
    type IntoIter = core::slice::Iter<'a, (&'scale str, Value<'scale>)>;

    fn into_iter(self) -> Self::IntoIter {
        if let Value::Object(ref vals) = *self {
            vals.iter()
        } else {
            debug_assert!(false); // This is not a good sign.
            todo!();
            // vec![].iter()
        }
    }
}

impl<'scale> Value<'scale> {
    pub fn get(&self, path: &str) -> Option<&Value> {
        let p: Vec<_> = path.split('.').collect();
        let mut cur = self;

        for pa in p {
            if let Value::Object(fields) = cur {
                if let Some((_, sub_val)) = fields.iter().find(|(name, _)| *name == pa) {
                    cur = sub_val;
                } else {
                    return None;
                }
            }
        }

        Some(cur)
    }

    // Assume that this is an object with just one field. TODO! rename only()
    pub fn only(&'scale self) -> Option<(&'scale str, &'scale Self)> {
        if let Self::Object(fields) = self {
            if fields.len() == 1 {
                let (name, val) = &fields[0];
                Some((name, val))
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn only2(&'scale self) -> Option<(&'scale str, &'scale str, &'scale Self)> {
        self.only()
            .and_then(|(head, tail)| tail.only().map(|(second, tail)| (head, second, tail)))
    }

    pub fn only3(&'scale self) -> Option<(&'scale str, &'scale str, &'scale str, &'scale Self)> {
        self.only2().and_then(|(first, second, tail)| {
            tail.only()
                .map(|(third, tail)| (first, second, third, tail))
        })
    }

    pub fn expect(&'scale self, expect1: &str) -> Option<&'scale Self> {
        self.only().and_then(|(head, tail)| {
            if head != expect1 {
                return None;
            }
            Some(tail)
        })
    }

    pub fn expect2(&'scale self, expect1: &str, expect2: &str) -> Option<&'scale Self> {
        self.expect(expect1).and_then(|tail| tail.expect(expect2))
    }

    pub fn expect3(
        &'scale self,
        expect1: &str,
        expect2: &str,
        expect3: &str,
    ) -> Option<&'scale Self> {
        self.expect2(expect1, expect2)
            .and_then(|tail| tail.expect(expect3))
    }

    pub fn expect4(
        &'scale self,
        expect1: &str,
        expect2: &str,
        expect3: &str,
        expect4: &str,
    ) -> Option<&'scale Self> {
        self.expect3(expect1, expect2, expect3)
            .and_then(|tail| tail.expect(expect4))
    }

    pub fn find(&'scale self, find1: &str) -> Option<&'scale Self> {
        if let Self::Object(fields) = self {
            for (field, val) in fields.iter() {
                if *field == find1 {
                    return Some(val);
                }
            }
        }
        None
    }

    pub fn find2(&'scale self, find1: &str, find2: &str) -> Option<&'scale Self> {
        self.find(find1).and_then(|val| {
            if let Self::Object(fields) = val {
                for (field, val) in fields.iter() {
                    if *field == find2 {
                        return Some(val);
                    }
                }
            }
            None
        })
    }
}

#[derive(Default)]
pub struct ValueBuilder<'scale> {
    root: Option<Value<'scale>>,
}

impl<'scale> ValueBuilder<'scale> {
    pub fn parse(
        data: &'scale [u8],
        top_type_id: u32,
        types: &'scale scale_info::PortableRegistry,
    ) -> Value<'scale> {
        let mut slf = ValueBuilder::<'scale>::default();
        crate::skeleton_decode(data, top_type_id, &mut slf, types);
        slf.root.take().unwrap()
    }

    fn append(
        path: &[(&'scale str, u32)],
        current: &mut Value<'scale>,
        new_field: &'scale str,
        new_val: Value<'scale>,
    ) {
        if let Value::<'scale>::Object(fields) = current {
            if path.is_empty() {
                // println!("appending path {:?} fin {:?}  / {:?} to {:?}",path, new_field, new_val, &fields);
                fields.push((new_field, new_val));
                return;
            }

            let ((head, head_ty), tail) = path.split_first().unwrap();
            for (field, child) in fields.iter_mut() {
                if field == head {
                    // println!("appending deeper new path {:?} | {:?}  / {:?} ", &tail, new_field, new_val);
                    ValueBuilder::append(tail, child, new_field, new_val);
                    return;
                }
            }
            // println!("appending path {:?} notfound {:?} adding {:?} | {:?}  / {:?} ", &tail, head, fields, new_field, new_val);

            fields.push((
                head,
                Value::Object(Box::new(vec![("_ty", Value::U32(*head_ty))])),
            ));
            let (_, new_current) = fields.last_mut().unwrap();
            ValueBuilder::append(tail, new_current, new_field, new_val);
        } else {
            panic!()
        }
    }

    #[cfg(not(feature = "bitvec"))]
    #[inline]
    fn parse_bitvec(data: &'scale [u8]) -> Option<Value> {
        Some(Value::Scale(data))
    }

    #[cfg(feature = "bitvec")]
    #[inline]
    fn parse_bitvec(mut data: &'scale [u8]) -> Option<Value> {
        assert_eq!(data.len(), 1, "bitvec size not suppored - please send pr.");
        use parity_scale_codec::Decode;
        Some(
             Value::Bits(Box::new(
                scale_value::BitSequence::decode(&mut data).unwrap())))
                // <bitvec::prelude::BitVec<u8, bitvec::prelude::Lsb0>
                // as
                // parity_scale_codec::Decode>::decode(&mut data).unwrap())))
    }
}

impl<'scale> super::VisitScale<'scale> for ValueBuilder<'scale> {
    fn visit(
        &mut self,
        current_path: &[(&'scale str, u32)],
        data: &'scale [u8],
        ty: &scale_info::Type<scale_info::form::PortableForm>,
        types: &PortableRegistry,
    ) {
        let new_val = match &ty.type_def {
            scale_info::TypeDef::Primitive(TypeDefPrimitive::Str) => Some(Value::Str(
                <&'scale str as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::Bool) => Some(Value::Bool(
                <bool as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::U8) => Some(Value::U8(
                <u8 as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::U16) => Some(Value::U16(
                <u16 as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::U32) => Some(Value::U32(
                <u32 as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::U64) => Some(Value::U64(
                <u64 as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            )),
            scale_info::TypeDef::Primitive(TypeDefPrimitive::U128) => Some(Value::U128(Box::new(
                <u128 as crate::borrow_decode::BorrowDecode>::borrow_decode(data),
            ))),

            TypeDef::Sequence(_) | TypeDef::Array(_) => {
                // Only hits here if it's u8, otherwise it's treated as an object with many fields.
                Some(Value::Scale(data))
            }
            TypeDef::BitSequence(_seq) => ValueBuilder::parse_bitvec(data),
            TypeDef::Compact(inner) => {
                let inner = types.resolve(inner.type_param.id).unwrap();
                match inner.type_def {
                    TypeDef::Primitive(TypeDefPrimitive::U32) => Some(Value::U32(
                        <Compact<u32> as crate::borrow_decode::BorrowDecode>::borrow_decode(data)
                            .into(),
                    )),
                    TypeDef::Primitive(TypeDefPrimitive::U64) => Some(Value::U64(
                        <Compact<u64> as crate::borrow_decode::BorrowDecode>::borrow_decode(data)
                            .into(),
                    )),
                    TypeDef::Primitive(TypeDefPrimitive::U128) => Some(Value::U128(Box::new(
                        <Compact<u128> as crate::borrow_decode::BorrowDecode>::borrow_decode(data)
                            .into(),
                    ))),
                    _ => panic!("unsupported {:?}", inner),
                }
            }
            _ => {
                panic!("skipping {:?}", ty);
            }
        };

        // place val in right location.
        let last = if self.root.is_none() {
            if current_path.is_empty() {
                self.root = new_val;
                return;
            }
            let (last, last_ty) = current_path.last().unwrap();
            self.root = Some(Value::Object(Box::new(vec![("_ty", Value::U32(*last_ty))])));
            last
        } else {
            let (last, _) = current_path.last().unwrap();
            last
        };

        // println!("appending {:?}  / {:?}", current_path, new_val);

        ValueBuilder::append(
            &current_path[..current_path.len() - 1],
            self.root.as_mut().unwrap(),
            last,
            new_val.unwrap(),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;

    #[test]
    fn test_iter() {
        let val = Value::Object(Box::new(vec![("0", Value::U32(0)), ("1", Value::U32(1))]));

        let it = val.into_iter();
        for i in it {
            println!("{:?}", i);
        }
    }

    #[test]
    #[cfg(feature = "display")]
    fn test_display() {
        let data = &[1, 2, 3, 4, 17, 18, 19, 20];
        let val = Value::Object(Box::new(vec![
            ("0", Value::U32(0)),
            ("1", Value::Scale(data)),
        ]));

        assert_eq!(
            r#"{0: U32(0), 1: Scale(0x0102030411121314)}"#,
            val.to_string()
        );

        let data = &[7; 200];
        let val = Value::Object(Box::new(vec![
            ("0", Value::U32(0)),
            ("1", Value::Scale(data)),
        ]));

        assert_eq!(
            r#"{0: U32(0), 1: Scale(0x07070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707070707...)}"#,
            val.to_string()
        );
    }
}
