use parity_scale_codec::Compact;
use parity_scale_codec::Decode;
use scale_info::form::PortableForm;
use scale_info::PortableRegistry;
use scale_info::Type;
use scale_info::{TypeDef, TypeDefPrimitive};
pub trait VisitScale<'scale> {
    // Visit value on current object
    fn visit(
        &mut self,
        path: &[(&'scale str, u32)],
        data: &'scale [u8],
        ty: &'scale Type<PortableForm>,
        types: &'scale PortableRegistry,
    );
}
pub mod borrow_decode;
pub mod value;
pub use value::{Value, ValueBuilder};
// use scale_decode::visitor::{self, TypeId};

#[macro_export]
macro_rules! descale {
    (struct $n:ident <$scale:lifetime> {
        $(#[path($path:literal)] $fieldname:ident: $t:ty,)+
    }) => {
        #[derive(Default)]
        struct $n<$scale> {
            $(pub $fieldname: $t,)+
            _tag: std::marker::PhantomData<&$scale [u8]>
        }

        impl <$scale> $n<$scale> {
            fn parse(data: &'scale [u8], top_type: UntrackedSymbol<TypeId>, types: &'scale scale_info::PortableRegistry) -> $n<$scale> {
                let mut slf = $n::<$scale>::default();
                crate::skeleton_decode(data, top_type.id(), &mut slf, types);
                slf
            }
        }

        impl <'scale> VisitScale<'scale> for $n<$scale> {
            fn visit(&mut self, current_path: &[(&'scale str,u32)], data: &'scale [u8], _ty: &'scale scale_info::Type<scale_info::form::PortableForm>, _types: &'scale PortableRegistry) {
                $(
                    let p: Vec<_> = $path.split('.').collect();//TODO: do earlier.
                    // println!("visited path {:?} == {:?}", current_path, p);
                    if current_path.len() == p.len() {
                        let same = current_path.iter().zip(p).all(|((seg,_), p_seg)| *seg == p_seg);
                        if same {
                            // println!("visited path found");
                            self.$fieldname = <$t as crate::borrow_decode::BorrowDecode>::borrow_decode(data);
                        }
                    }
                )+
            }
        }
    };
}

/// Walk the bytes with knowledge of the type and metadata and provide slices
/// to the visitor that it can optionally decode.
pub fn skeleton_decode<'scale>(
    data: &'scale [u8],
    ty_id: u32,
    visitor: &mut impl VisitScale<'scale>,
    types: &'scale PortableRegistry,
) {
    let id = ty_id;
    let ty = types.resolve(id).unwrap();
    let vec: Vec<(&'scale str, u32)> = vec![];
    let cursor = &mut &*data;
    semi_decode_aux(vec, cursor, ty, id, visitor, types);
}

// struct BorrowVisitor<'scale> {
//     raw_scale : &'scale[u8]
// }

// impl <'scale> scale_decode::visitor::Visitor for BorrowVisitor<'scale> {
//     type Value = crate::Value<'scale>;
//     type Error = scale_decode::visitor::DecodeError;

//     fn visit_bool(self, value: bool, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::Bool(value))
// 	}
// 	fn visit_char(self, value: char, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::Char(value))
// 	}
// 	fn visit_u8(self, value: u8, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::U8(value))
// 	}
// 	fn visit_u16(self, value: u16, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::U16(value))
// 	}
// 	fn visit_u32(self, value: u32, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::U32(value))
// 	}
// 	fn visit_u64(self, value: u64, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::U64(value))
// 	}
// 	fn visit_u128(self, value: u128, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::U128(Box::new(value)))
// 	}
// 	fn visit_u256(self, value: &[u8; 32], _type_id: TypeId) -> Result<Self::Value, Self::Error> {
//         todo!("lifetime issues")
// 		// Ok(Value::U256(value))
// 	}
// 	fn visit_i8(self, value: i8, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::I8(value))
// 	}
// 	fn visit_i16(self, value: i16, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::I16(value))
// 	}
// 	fn visit_i32(self, value: i32, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::I32(value))
// 	}
// 	fn visit_i64(self, value: i64, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::I64(value))
// 	}
// 	fn visit_i128(self, value: i128, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::I128(Box::new(value)))
// 	}
// 	fn visit_i256(self, value: &[u8; 32], _type_id: TypeId) -> Result<Self::Value, Self::Error> 
//         // where 'scale : 'a
//     {
// 		// Ok(Value::I256(value))
//         todo!("lifetime issues")
// 	}
// 	fn visit_sequence(
// 		self,
// 		_value: &mut scale_decode::visitor::Sequence,
// 		_type_id: TypeId,
// 	) -> Result<Self::Value, Self::Error> {
// 		todo!();
// 	}
// 	fn visit_composite(
// 		self,
// 		_value: &mut scale_decode::visitor::Composite,
// 		_type_id: TypeId,
// 	) -> Result<Self::Value, Self::Error> {
// 		todo!();
// 	}

// 	fn visit_tuple(self, _value: &mut scale_decode::visitor::Tuple, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		todo!();
// 	}

// 	fn visit_str<'a>(self, value: scale_decode::visitor::Str<'a>, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
// 		Ok(Value::Str(value.as_str().unwrap())) //TODO
// 	}

// 	fn visit_array(self, _value: &mut scale_decode::visitor::Array, _type_id: TypeId) -> Result<Self::Value, Self::Error> {
//         todo!();
// //		Ok(())
// 	}

// 	fn visit_variant(
// 		self,
// 		_value: &mut scale_decode::visitor::Variant,
// 		_type_id: TypeId,
// 	) -> Result<Self::Value, Self::Error> {
// 	      todo!();
// //		Ok(())
// 	}

// 	fn visit_bitsequence(
// 		self,
// 		value: &mut scale_decode::visitor::BitSequence,
// 		_type_id: TypeId,
// 	) -> Result<Self::Value, Self::Error> {
//         #[cfg(not(feature = "bitvec"))]
//         panic!("Unsupported: use bitvec feature to turn this support on.");
//         #[cfg(feature = "bitvec")]
// 		Ok(Value::Bits(value).unwrap())
// 	}
// }


static NUMS: &[&str] = &["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];
fn semi_decode_aux<'scale, V: VisitScale<'scale>>(
    mut stack: Vec<(&'scale str, u32)>,
    data: &mut &'scale [u8],
    ty: &'scale Type<PortableForm>,
    id: u32,
    visitor: &mut V,
    types: &'scale PortableRegistry,
) -> Vec<(&'scale str, u32)> {
    println!("decode {:#?} - left {}", ty.type_def, data.len());
    let original_len = data.len();
    match &ty.type_def {
        TypeDef::Composite(inner) => {
            for (i, field) in inner.fields.iter().enumerate() {
                let id = field.ty.id;
                let field_ty = types.resolve(id).unwrap();
                let s: &'scale str = NUMS[i];
                let fieldname: &'scale str = field.name.as_ref().map(|s| s.as_str()).unwrap_or(s);
                stack.push((fieldname, id));
                stack = semi_decode_aux(stack, data, field_ty, id, visitor, types);
                stack.pop();
            }
        }
        TypeDef::Variant(var) => {
            let (&discriminant, data_new) = data.split_first().unwrap();
            *data = data_new;
            let variant = var
                .variants
                .iter()
                .find(|v| v.index == discriminant)
                .unwrap();

            stack.push((&variant.name, id));
            for (i, field) in variant.fields.iter().enumerate() {
                let id = field.ty.id;
                let field_ty = types.resolve(id).unwrap();
                let s: &'scale str = NUMS[i];

                let fieldname: &'scale str = if let Some(ref name) = field.name {
                    name.as_str()
                } else {
                    s
                };
                stack.push((fieldname, id));
                stack = semi_decode_aux(stack, data, field_ty, id, visitor, types);
                stack.pop();
            }
            stack.pop();
        }
        TypeDef::Primitive(TypeDefPrimitive::Str) => {
            let len: u32 = Compact::<u32>::decode(data).unwrap().into();
            let len = len as usize;
            visitor.visit(&stack, &data[..len], ty, types);
            *data = &data[len..];
        }
        TypeDef::Primitive(TypeDefPrimitive::Bool) => {
            // let size = ty..encoded_fixed_size().unwrap();
            visitor.visit(&stack, &data[..1], ty, types);
            *data = &data[1..];
        }
        TypeDef::Primitive(TypeDefPrimitive::U8) => {
            const LEN: usize = 1;
            visitor.visit(&stack, &data[..LEN], ty, types);
            *data = &data[LEN..];
        }
        TypeDef::Primitive(TypeDefPrimitive::U16) => {
            const LEN: usize = 2;
            visitor.visit(&stack, &data[..LEN], ty, types);
            *data = &data[LEN..];
        }
        TypeDef::Primitive(TypeDefPrimitive::U32) => {
            const LEN: usize = 4;
            visitor.visit(&stack, &data[..LEN], ty, types);
            *data = &data[LEN..];
        }
        TypeDef::Primitive(TypeDefPrimitive::U64) => {
            const LEN: usize = 8;
            visitor.visit(&stack, &data[..LEN], ty, types);
            *data = &data[LEN..];
        }
        TypeDef::Primitive(TypeDefPrimitive::U128) => {
            const LEN: usize = 16;
            visitor.visit(&stack, &data[..LEN], ty, types);
            *data = &data[LEN..];
        }
        TypeDef::Sequence(seq) => {
            let len: u64 = Compact::<u64>::decode(data).unwrap().into();
            let ty_id = seq.type_param;
            let ty_inner = types.resolve(ty_id.id).unwrap();
            if ty_inner.type_def == TypeDef::Primitive(TypeDefPrimitive::U8) {
                visitor.visit(&stack, &data[..len as usize], ty, types);
                *data = &data[usize::try_from(len).unwrap()..];
            } else {
                println!("seq len = {}", len);
                for i in NUMS.iter().take(len as usize) {
                    // println!("i = {}", i);println!("bytes left to decode start: {:?}", &data);
                    stack.push((i, ty_id.id));
                    // NB: this call must move the data slice onwards.
                    stack = semi_decode_aux(stack, data, ty_inner, ty_id.id, visitor, types);
                    // println!("bytes left to decode end  : {:?}", &data);
                    stack.pop();
                }
            }
        }
        TypeDef::Array(arr) => {
            let len: u32 = arr.len;
            let ty_id = arr.type_param;
            let ty_inner = types.resolve(ty_id.id).unwrap();
            if ty_inner.type_def == TypeDef::Primitive(TypeDefPrimitive::U8) {
                visitor.visit(&stack, &data[..len as usize], ty, types);
                *data = &data[len as usize..];
            } else {
                println!("seq len = {}", len);
                for i in NUMS.iter().take(len as usize) {
                    // println!("i = {}", i);println!("bytes left to decode start: {:?}", &data);
                    stack.push((i, ty_id.id));
                    // NB: this call must move the data slice onwards.
                    stack = semi_decode_aux(stack, data, ty_inner, ty_id.id, visitor, types);
                    // println!("bytes left to decode end  : {:?}", &data);
                    stack.pop();
                }
            }
        }
        TypeDef::BitSequence(seq) => {
            // assert_eq!(seq.bit_order_type(), bitvec::prelude::Lsb0);
            let ty_inner = types.resolve(seq.bit_store_type.id).unwrap();
            match ty_inner.type_def {
                TypeDef::Primitive(TypeDefPrimitive::U8) => {
                    visitor.visit(&stack, &data[..1], ty, types);
                    *data = &data[1..];
                }
                _ => panic!("unsupported bitvec size - send PR please."),
            }
        }
        TypeDef::Compact(inner) => {
            let ty_inner = types.resolve(inner.type_param.id).unwrap();

            match ty_inner.type_def {
                TypeDef::Primitive(TypeDefPrimitive::U32) => {
                    visitor.visit(&stack, data, ty, types);
                    Compact::<u32>::skip(data).unwrap();
                }
                TypeDef::Primitive(TypeDefPrimitive::U64) => {
                    visitor.visit(&stack, data, ty, types);
                    Compact::<u64>::skip(data).unwrap();
                }
                TypeDef::Primitive(TypeDefPrimitive::U128) => {
                    visitor.visit(&stack, data, ty, types);
                    Compact::<u128>::skip(data).unwrap();
                }
                _ => panic!(
                    "unsupported compact size - send PR please. {:?}",
                    ty_inner.type_def
                ),
            }
            //  panic!("don't understand a {:?}", ty_inner.type_def());
        }
        _ => {
            panic!("don't understand a {:?}", ty.type_def);
        }
    }
    assert!(data.len() < original_len, "failed to make any progress!");
    stack
}

#[cfg(test)]
mod tests {
    use super::value::{Value, ValueBuilder};
    use crate::VisitScale;
    use parity_scale_codec::*;
    use scale_info::interner::UntrackedSymbol;
    use scale_info::prelude::any::TypeId;
    use scale_info::PortableRegistry;
    use wasm_bindgen_test::*;

    /// Given a type definition, return the PortableType and PortableRegistry
    /// that our decode functions expect.
    fn make_type<T: scale_info::TypeInfo + 'static>() -> (UntrackedSymbol<TypeId>, PortableRegistry)
    {
        let m = scale_info::MetaType::new::<T>();
        let mut types = scale_info::Registry::new();
        let id = types.register_type(&m);
        let portable_registry: PortableRegistry = types.into();

        (id, portable_registry)
    }

    #[wasm_bindgen_test]
    #[test]
    fn bool_test() {
        let val = false;
        let encoded = val.encode();

        let (id, types) = make_type::<bool>();

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(val, Value::Bool(false));
    }

    // #[wasm_bindgen_test]
    // #[test]
    // #[cfg(feature = "bitvec")]
    // fn bitvec_test() {
    //     use bitvec::prelude::*;
    //     let val = bitvec![u8, Msb0;];
    //     let encoded = val.encode();

    //     let (id, types) = make_type::<BitVec<u8, bitvec::order::Lsb0>>();

    //     let val = ValueBuilder::parse(&encoded, id, &types);
    //     assert_eq!(val, Value::Bits(Box::new(bitvec![u8, Lsb0;])));
    // }

    // #[wasm_bindgen_test]
    // #[test]
    // #[cfg(not(feature = "bitvec"))]
    // fn bitvec_test2() {
    //     // use bitvec::prelude::*;
    //     let val = bitvec![u8, Msb0;];
    //     let encoded = val.encode();

    //     let (id, types) = make_type::<BitVec<u8, bitvec::order::Lsb0>>();

    //     let val = ValueBuilder::parse(&encoded, id.id(), &types);
    //     assert_eq!(val, Value::Scale(&[0]));
    // }

    #[wasm_bindgen_test]
    #[test]
    fn string_test() {
        let val = "hello string";
        let encoded = val.encode();

        let (id, types) = make_type::<&str>();

        let value = ValueBuilder::parse(&encoded, id.id(), &types);
        if let Value::Str(inner) = value {
            assert_eq!(val, inner);
        } else {
            panic!()
        }
    }

    #[wasm_bindgen_test]
    #[test]
    fn struct_test() {
        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        struct X {
            val: bool,
            name: String,
        }
        let val = X {
            val: true,
            name: "hi val".into(),
        };
        let encoded = val.encode();

        let (id, types) = make_type::<X>();

        descale! {
            struct XParse<'scale> {
                #[path("val")]
                named_bool: bool,
                #[path("name")]
                named_bool2: &'scale str,
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.named_bool, true);
        assert_eq!(xx.named_bool2, "hi val");

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(1)),
                ("val", Value::Bool(true)),
                ("name", Value::Str("hi val"))
            ]))
        );
    }

    #[wasm_bindgen_test]
    #[test]
    fn enum_test() {
        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        enum X {
            A,
            B(u32, u64),
            C { val: bool },
        }
        let val = X::C { val: true };
        let encoded = val.encode();

        let (id, types) = make_type::<X>();

        descale! {
            struct XParse<'scale> {
                #[path("C.val")]
                named_bool: bool,
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.named_bool, true);

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(3)),
                (
                    "C",
                    Value::Object(Box::new(vec![
                        ("_ty", Value::U32(0)),
                        ("val", Value::Bool(true))
                    ]),)
                )
            ]))
        );
    }

    #[wasm_bindgen_test]
    #[test]
    fn tuple_test() {
        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        enum X {
            A,
            B(u32, u64),
            C { val: bool },
        }
        let val = X::B(10, 20);
        let encoded = val.encode();

        let (id, types) = make_type::<X>();

        descale! {
            struct XParse<'scale> {
                #[path("B.0")]
                val: u32,
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.val, 10);

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(1)),
                (
                    "B",
                    Value::Object(Box::new(vec![
                        ("_ty", Value::U32(0)),
                        ("0", Value::U32(10)),
                        ("1", Value::U64(20))
                    ]),)
                )
            ]))
        );
    }

    #[wasm_bindgen_test]
    #[test]
    fn slice_u8_test() {
        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        struct X {
            more_scale: Vec<u8>,
        }
        let val = X {
            more_scale: vec![1, 2, 3, 4],
        };
        let encoded = val.encode();

        let (id, types) = make_type::<X>();

        descale! {
            struct XParse<'scale> {
                #[path("more_scale")]
                uncopied_bytes: &'scale [u8],
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.uncopied_bytes, vec![1, 2, 3, 4].as_slice());

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(1)),
                ("more_scale", Value::Scale(&[1, 2, 3, 4])),
            ]))
        );
    }

    #[wasm_bindgen_test]
    #[test]
    fn num_tests() {
        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        struct X {
            a: u8,
            b: u16,
            c: u32,
            d: u64,
            e: u128,
        }
        let val = X {
            a: 1,
            b: 2,
            c: 3,
            d: 4,
            e: 5,
        };
        let encoded = val.encode();

        let (id, types) = make_type::<X>();

        descale! {
            struct XParse<'scale> {
                #[path("a")]
                a: u8,
                #[path("b")]
                b: u16,
                #[path("c")]
                c: u32,
                #[path("d")]
                d: u64,
                #[path("e")]
                e: u128,
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.a, 1);
        assert_eq!(xx.b, 2);
        assert_eq!(xx.c, 3);
        assert_eq!(xx.d, 4);
        assert_eq!(xx.e, 5);

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(1)),
                ("a", Value::U8(1)),
                ("b", Value::U16(2)),
                ("c", Value::U32(3)),
                ("d", Value::U64(4)),
                ("e", Value::U128(Box::new(5)))
            ]))
        );
    }

    #[wasm_bindgen_test]
    #[test]
    fn array_test() {
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        struct Y {
            outer: Vec<X>, // 8 len
        }

        // Only try and decode the bool
        #[derive(Decode, Encode, scale_info::TypeInfo)]
        struct X {
            val: bool,
            name: String,
        }
        let val = X {
            val: true,              // 1
            name: "skip me".into(), // 28 len then 115, 107, 105, 112, 32, 109, 101
        };
        let val2 = X {
            val: false,              // 0
            name: "skip meh".into(), // 28 len then 115, 107, 105, 112, 32, 109, 101, h
        };
        let y = Y {
            outer: vec![val, val2],
        };
        let encoded = y.encode();
        println!("bytes {:?}", encoded);

        let (id, types) = make_type::<Y>();

        descale! {
            struct XParse<'scale> {
                #[path("outer.0.val")]
                named_bool: bool,
                #[path("outer.1.name")]
                named_bool2: &'scale str,
            }
        };
        let xx = XParse::parse(&encoded[..], id, &types);
        assert_eq!(xx.named_bool, true);
        assert_eq!(xx.named_bool2, "skip meh");

        let val = ValueBuilder::parse(&encoded, id.id(), &types);
        assert_eq!(
            val,
            Value::Object(Box::new(vec![
                ("_ty", Value::U32(3)),
                (
                    "outer",
                    Value::Object(Box::new(vec![
                        ("_ty", Value::U32(1)),
                        (
                            "0",
                            Value::Object(Box::new(vec![
                                ("_ty", Value::U32(2)),
                                ("val", Value::Bool(true)),
                                ("name", Value::Str("skip me"))
                            ]))
                        ),
                        (
                            "1",
                            Value::Object(Box::new(vec![
                                ("_ty", Value::U32(2)),
                                ("val", Value::Bool(false)),
                                ("name", Value::Str("skip meh"))
                            ]))
                        ),
                    ]))
                )
            ]))
        );
    }

    #[test]
    fn test_value() {
        assert_eq!(std::mem::size_of::<super::value::Value>(), 24); // 16 in wasm32
        assert_eq!(std::mem::size_of::<u128>(), 16);
    }
}
