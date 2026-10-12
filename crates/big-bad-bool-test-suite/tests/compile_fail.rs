use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn invalid_bitpack_schemas_fail_to_compile() {
    let cases = [
        CompileFailCase {
            name: "bits_zero",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(0)]
                    field: u8,
                }
            "#,
            expected_stderr: "`#[bits(0)]` is not valid",
        },
        CompileFailCase {
            name: "signed_field",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    field: i8,
                }
            "#,
            expected_stderr: "unsupported storage type",
        },
        CompileFailCase {
            name: "missing_bits_attr",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    field: u8,
                }
            "#,
            expected_stderr: "each bitpack field requires `#[bits(n)]`",
        },
        CompileFailCase {
            name: "fields_exceed_storage",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(5)]
                    first: u8,
                    #[bits(5)]
                    second: u8,
                }
            "#,
            expected_stderr: "bit field overflow",
        },
        CompileFailCase {
            name: "field_type_too_narrow",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u16)]
                pub struct Bad {
                    #[bits(9)]
                    field: u8,
                }
            "#,
            expected_stderr: "cannot represent that many bits",
        },
        CompileFailCase {
            name: "unsupported_storage",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = i16)]
                pub struct Bad {
                    #[bits(3)]
                    field: u8,
                }
            "#,
            expected_stderr: "unsupported storage type",
        },

        CompileFailCase {
            name: "label_value_out_of_range",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 7, too_large = 8)]
                    field: u8,
                }
            "#,
            expected_stderr: "only accepts values up to 7",
        },
        CompileFailCase {
            name: "duplicate_label_name",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(same = 1, same = 2)]
                    field: u8,
                }
            "#,
            expected_stderr: "duplicate label name `same`",
        },
        CompileFailCase {
            name: "duplicate_label_value",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(first = 1, second = 1)]
                    field: u8,
                }
            "#,
            expected_stderr: "duplicate label value 1",
        },

        CompileFailCase {
            name: "duplicate_field_id",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(2)]
                    #[field_id(1)]
                    first: u8,
                    #[bits(2)]
                    #[field_id(1)]
                    second: u8,
                }
            "#,
            expected_stderr: "duplicate field id 1",
        },
        CompileFailCase {
            name: "field_id_zero",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(2)]
                    #[field_id(0)]
                    field: u8,
                }
            "#,
            expected_stderr: "field ids are 1-based",
        },
        CompileFailCase {
            name: "reserved_range_out_of_range",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[reserved(8)]
                    field: u8,
                }
            "#,
            expected_stderr: "reserved range 8..=8",
        },
        CompileFailCase {
            name: "invalid_range_out_of_range",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[invalid(4..=8)]
                    field: u8,
                }
            "#,
            expected_stderr: "invalid range 4..=8",
        },
        CompileFailCase {
            name: "default_out_of_range",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[default(8)]
                    field: u8,
                }
            "#,
            expected_stderr: "default value 8",
        },
        CompileFailCase {
            name: "flag_bit_out_of_range",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[flags(too_high = 3)]
                    field: u8,
                }
            "#,
            expected_stderr: "uses bit 3 but field width is 3",
        },
        CompileFailCase {
            name: "duplicate_flag_name",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[flags(same = 0, same = 1)]
                    field: u8,
                }
            "#,
            expected_stderr: "duplicate flag name `same`",
        },
        CompileFailCase {
            name: "duplicate_flag_bit",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[flags(first = 0, second = 0)]
                    field: u8,
                }
            "#,
            expected_stderr: "duplicate flag bit 0",
        },
        CompileFailCase {
            name: "alias_unknown_label",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 1)]
                    #[aliases(missing = ["old"])]
                    field: u8,
                }
            "#,
            expected_stderr: "alias target `missing`",
        },
        CompileFailCase {
            name: "deprecated_unknown_label",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 1)]
                    #[deprecated(missing, since = "0.1", note = "bad")]
                    field: u8,
                }
            "#,
            expected_stderr: "deprecated label `missing`",
        },
        CompileFailCase {
            name: "requires_unknown_label",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 1)]
                    #[requires(missing => other.value)]
                    field: u8,
                }
            "#,
            expected_stderr: "requires source label `missing`",
        },
        CompileFailCase {
            name: "conflicts_needs_two_labels",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 1)]
                    #[conflicts(ok)]
                    field: u8,
                }
            "#,
            expected_stderr: "needs at least two labels",
        },
        CompileFailCase {
            name: "conflicts_unknown_label",
            source: r#"
                use big_bad_bool::bitpack;

                #[bitpack(storage = u8)]
                pub struct Bad {
                    #[bits(3)]
                    #[labels(ok = 1, other = 2)]
                    #[conflicts(ok, missing)]
                    field: u8,
                }
            "#,
            expected_stderr: "conflict label `missing`",
        },
    ];

    for case in cases {
        assert_compile_fails(case);
    }
}

struct CompileFailCase {
    name: &'static str,
    source: &'static str,
    expected_stderr: &'static str,
}

fn assert_compile_fails(case: CompileFailCase) {
    let crate_dir = make_case_crate(case.name, case.source);
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());

    let output = Command::new(cargo)
        .arg("check")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(crate_dir.join("Cargo.toml"))
        .output()
        .expect("failed to invoke cargo for compile-fail fixture");

    assert!(
        !output.status.success(),
        "compile-fail case `{}` unexpectedly compiled successfully",
        case.name
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(case.expected_stderr),
        "compile-fail case `{}` did not include expected stderr `{}`.\n\nactual stderr:\n{}",
        case.name,
        case.expected_stderr,
        stderr
    );
}

fn make_case_crate(name: &str, source: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "big_bad_bool_compile_fail_{}_{}",
        std::process::id(),
        name
    ));

    if root.exists() {
        fs::remove_dir_all(&root).expect("failed to remove previous compile-fail fixture");
    }

    fs::create_dir_all(root.join("src")).expect("failed to create compile-fail fixture");

    fs::write(
        root.join("Cargo.toml"),
        format!(
            r#"[package]
name = "big_bad_bool_compile_fail_{name}"
version = "0.0.0"
edition = "2021"
publish = false

[dependencies]
big_bad_bool_core = {{ package = "big-bad-bool-core", version = "0.1.0" }}
big_bad_bool = {{ package = "big-bad-bool", version = "0.1.0" }}
"#
        ),
    )
    .expect("failed to write compile-fail Cargo.toml");

    fs::write(root.join("src/lib.rs"), source).expect("failed to write compile-fail source");

    root
}
