# scale-borrow

Decode scale in a dynamic way but reasonably efficiently because rust can.

## Who this crate is not for

If you are targeting a few specific parachains --> use subxt

If you are targeting lots of parachains and just need something easy --> use scale-value (or https://github.com/virto-network/scales)

## Goals

   * fun, pleasent
   * efficient
   * few deps, fast to compile (28 deps)
   * wasm compatible - currently using integritee's fork of frame-metadata to achieve this.

## How to use

### Pic 'n Mix

You can pick and mix the bits you care about into a struct:

```rust
   descale! {
      struct MyStruct<'scale> {
            #[path("outer.0.val")]
            named_bool: bool,
            #[path("outer.1.name")]
            named_bool2: &'scale str,
      }
   };
   let my_struct = MyStruct::parse(&encoded[..], top_type_id, &types);
```

alternatively there's Value.

### All the world is a `Value`

```rust
   let val = ValueBuilder::parse(&encoded, top_type_id, &types);
   assert_eq!(
      val,
      Value::Object(Box::new(vec![
         ("_ty", Value::U32(1)),
         ("val", Value::Bool(true)),
         ("name", Value::Str("hi val"))
      ]))
   );
```

The `_ty` field is the type of the struct. Tuples and arrays have field names 0, 1, 2 etc.

## Status

Very experimental

TODO non-panic error handling,
