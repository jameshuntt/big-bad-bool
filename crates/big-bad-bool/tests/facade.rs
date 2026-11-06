//! The facade root with any feature set: `#[bitpack]` and the core names
//! work the same whether or not `program` is enabled. The program layer adds
//! only the `program` module.

use big_bad_bool::{bitpack, BitFieldSpec, PackedWord};

#[bitpack(storage = u8)]
pub struct Pair {
    #[bits(3)]
    pub low: u8,
    #[bits(5)]
    pub high: u8,
}

#[test]
fn bitpack_and_core_names_are_at_the_root() {
    let fields: &[BitFieldSpec] = <Pair as PackedWord>::FIELDS;
    let names: Vec<&str> = fields.iter().map(|f| f.name).collect();
    assert_eq!(names, ["low", "high"]);
    assert_eq!(big_bad_bool::core::value_mask(5), 0b1_1111);
}
