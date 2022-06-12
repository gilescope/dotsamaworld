use desub_current::value::*;
#[allow(dead_code)]

use std::collections::HashMap;

pub fn print_val<T>(dbg: &desub_current::ValueDef<T>) {
    match dbg {
        desub_current::ValueDef::BitSequence(..) => {
            println!("bit sequence");
        }
        desub_current::ValueDef::Composite(inner) => match inner {
            Composite::Named(fields) => {
                println!("named composit (");
                for (n, f) in fields {
                    print!("{n}");
                    print_val(&f.value);
                }
                println!(")");
            }
            Composite::Unnamed(fields) => {
                println!("un named composita(");

                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(_)))
                    && fields.len() > 1
                {
                    println!(" << primitive array >> ");
                } else {
                    for f in fields.iter() {
                        print_val(&f.value);
                    }
                }
                println!(")");
            }
        },
        desub_current::ValueDef::Primitive(..) => {
            println!("primitiv");
        }
        desub_current::ValueDef::Variant(Variant { name, values }) => {
            println!("variatt {name} (");
            match values {
                Composite::Named(fields) => {
                    println!("named composit (");
                    for (n, f) in fields {
                        print!("{n}");
                        print_val(&f.value);
                    }
                    println!(")");
                }
                Composite::Unnamed(fields) => {
                    println!("un named composita(");
                    for f in fields.iter() {
                        print_val(&f.value);
                    }
                    println!(")");
                }
            }
            println!("variatt end {name} )");
        }
    }
}

// THere's better ways but crazy levels of matching...
pub fn flattern<T>(
    dbg: &desub_current::ValueDef<T>,
    location: &str,
    mut results: &mut HashMap<String, String>,
) {
    match dbg {
        desub_current::ValueDef::BitSequence(..) => {
            // println!("bitseq skipped");
        }
        desub_current::ValueDef::Composite(inner) => match inner {
            Composite::Named(fields) => {
                for (n, f) in fields {
                    flattern(&f.value, &format!("{}.{}", location, n), &mut results);
                }
            }
            Composite::Unnamed(fields) => {
                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        format!("{}", location),
                        hex::encode(
                            fields
                                .iter()
                                .map(|f| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields.iter().enumerate() {
                        flattern(&f.value, &format!("{}.{}", location, n), &mut results);
                    }
                }
            }
        },
        desub_current::ValueDef::Primitive(Primitive::U8(val)) => {
            results.insert(location.to_string(), val.to_string());
        }
        desub_current::ValueDef::Primitive(Primitive::U32(val)) => {
            results.insert(location.to_string(), val.to_string());
        }
        desub_current::ValueDef::Primitive(..) => {
            // println!("primitiv skipped");
        }
        desub_current::ValueDef::Variant(Variant { name, values }) => match values {
            Composite::Named(fields) => {
                if fields
                    .iter()
                    .all(|(_name, f)| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        format!("{},{}", name, location),
                        hex::encode(
                            fields
                                .iter()
                                .map(|(_, f)| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields {
                        flattern(
                            &f.value,
                            &format!("{}.{}.{}", location, name, n),
                            &mut results,
                        );
                    }
                }
            }
            Composite::Unnamed(fields) => {
                if fields
                    .iter()
                    .all(|f| matches!(f.value, ValueDef::Primitive(Primitive::U8(_))))
                    && fields.len() > 1
                {
                    results.insert(
                        location.to_string(),
                        hex::encode(
                            fields
                                .iter()
                                .map(|f| {
                                    if let ValueDef::Primitive(Primitive::U8(byte)) = f.value {
                                        byte
                                    } else {
                                        panic!();
                                    }
                                })
                                .collect::<Vec<_>>(),
                        ),
                    );
                } else {
                    for (n, f) in fields.iter().enumerate() {
                        flattern(
                            &f.value,
                            &format!("{}.{}.{}", location, name, n),
                            &mut results,
                        );
                    }
                }
            }
        },
    }
}
