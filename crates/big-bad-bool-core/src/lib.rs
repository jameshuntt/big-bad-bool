//! `big-bad-bool-core` is the small public surface for generated packed words.
//!
//! The macro owns the bit math. This crate owns the stable runtime vocabulary:
//! field metadata, schema lookup, raw-word conversion, and shared traits used by
//! higher-level dispatch/negotiation crates.

/// Error returned by strict checked constructors and validators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitpackError {
    /// Specific failure category.
    pub kind: BitpackErrorKind,
}

impl BitpackError {
    /// A field value is too large for its declared bit width.
    #[must_use]
    pub const fn field_overflow(field: &'static str, value: u128, max: u128) -> Self {
        Self {
            kind: BitpackErrorKind::FieldOverflow { field, value, max },
        }
    }

    /// A raw `u128` value is too large to fit into the native storage type.
    #[must_use]
    pub const fn raw_overflow(raw: u128, max: u128) -> Self {
        Self {
            kind: BitpackErrorKind::RawOverflow { raw, max },
        }
    }

    /// A raw value contains bits outside the declared packed layout.
    #[must_use]
    pub const fn raw_layout_violation(raw: u128, mask: u128) -> Self {
        Self {
            kind: BitpackErrorKind::RawLayoutViolation { raw, mask },
        }
    }
}

impl core::fmt::Display for BitpackError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            BitpackErrorKind::FieldOverflow { field, value, max } => {
                write!(f, "field `{field}` value {value} exceeds max {max}")
            }
            BitpackErrorKind::RawOverflow { raw, max } => {
                write!(f, "raw value {raw} exceeds storage max {max}")
            }
            BitpackErrorKind::RawLayoutViolation { raw, mask } => {
                write!(
                    f,
                    "raw value {raw:#x} contains bits outside layout mask {mask:#x}"
                )
            }
        }
    }
}

impl std::error::Error for BitpackError {}

/// Specific failure category for strict bitpack operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BitpackErrorKind {
    /// A named field value exceeds the field's maximum representable value.
    FieldOverflow {
        /// Field name as declared in the packed schema.
        field: &'static str,
        /// Rejected value.
        value: u128,
        /// Maximum value accepted by the field.
        max: u128,
    },
    /// A raw value cannot fit into the generated storage type.
    RawOverflow {
        /// Rejected raw value.
        raw: u128,
        /// Maximum value accepted by the native storage type.
        max: u128,
    },
    /// A raw value fits in storage but sets bits outside the declared layout.
    RawLayoutViolation {
        /// Rejected raw value.
        raw: u128,
        /// Mask covering all valid packed-layout bits.
        mask: u128,
    },
}

/// Static metadata for a generated bit field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitFieldSpec {
    /// Stable 1-based field id used by tools/WASM. Generated from source order
    /// unless `#[field_id(n)]` is supplied.
    pub id: u16,
    /// Field name as written in the source struct.
    pub name: &'static str,
    /// Starting bit offset, least-significant-bit first.
    pub shift: u32,
    /// Number of bits owned by this field.
    pub width: u32,
    /// Mask at the raw storage position.
    pub mask: u128,
    /// Mask before shifting.
    pub value_mask: u128,
    /// Human-readable description supplied by `#[describe("...")]`, or empty.
    pub description: &'static str,
    /// Default value supplied by `#[default(n)]`, or `None`.
    pub default: Option<u128>,
}

impl BitFieldSpec {
    /// Maximum value that fits in this field.
    #[must_use]
    pub const fn max_value(self) -> u128 {
        self.value_mask
    }

    /// Returns true when `value` can be stored in this field.
    #[must_use]
    pub const fn accepts(self, value: u128) -> bool {
        value <= self.value_mask
    }
}

/// Human-readable label for a concrete field value.
///
/// Labels can be generated directly by `#[bitpack]` using field-level
/// `#[labels(name = value, ...)]` attributes, or supplied externally by higher
/// level protocol/spec crates. The generated bit math does not depend on labels;
/// labels are semantic metadata for docs, dispatch reports, CLIs, WASM schema
/// inspection, and builder UIs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueLabel {
    /// Field name as declared in the packed schema.
    pub field: &'static str,
    /// Concrete value within the field.
    pub value: u128,
    /// Stable label name for this field value.
    pub label: &'static str,
    /// Optional human-readable description. Empty when generated from the
    /// shorthand macro syntax.
    pub description: &'static str,
}

impl ValueLabel {
    /// Creates a label with an explicit description.
    #[must_use]
    pub const fn new(
        field: &'static str,
        value: u128,
        label: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            field,
            value,
            label,
            description,
        }
    }

    /// Creates a label without a description. This is what the macro-generated
    /// `#[labels(name = value)]` shorthand uses.
    #[must_use]
    pub const fn named(field: &'static str, value: u128, label: &'static str) -> Self {
        Self {
            field,
            value,
            label,
            description: "",
        }
    }
}

/// Reserved/invalid range kind generated from `#[reserved(...)]` and `#[invalid(...)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueRangeKind {
    Reserved,
    Invalid,
}

/// A semantic value range on a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueRange {
    pub field: &'static str,
    pub kind: ValueRangeKind,
    pub min: u128,
    pub max: u128,
}

impl ValueRange {
    #[must_use]
    pub const fn reserved(field: &'static str, min: u128, max: u128) -> Self {
        Self {
            field,
            kind: ValueRangeKind::Reserved,
            min,
            max,
        }
    }

    #[must_use]
    pub const fn invalid(field: &'static str, min: u128, max: u128) -> Self {
        Self {
            field,
            kind: ValueRangeKind::Invalid,
            min,
            max,
        }
    }

    #[must_use]
    pub const fn contains(self, value: u128) -> bool {
        value >= self.min && value <= self.max
    }
}

/// Named flag bit inside a field generated from `#[flags(name = bit, ...)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BitFlag {
    pub field: &'static str,
    pub bit: u32,
    pub mask: u128,
    pub name: &'static str,
    pub description: &'static str,
}

impl BitFlag {
    #[must_use]
    pub const fn new(
        field: &'static str,
        bit: u32,
        name: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            field,
            bit,
            mask: 1u128 << bit,
            name,
            description,
        }
    }
}

/// Alias for a generated label, used for migrations/spec vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueAlias {
    pub field: &'static str,
    pub label: &'static str,
    pub alias: &'static str,
}

impl ValueAlias {
    #[must_use]
    pub const fn new(field: &'static str, label: &'static str, alias: &'static str) -> Self {
        Self {
            field,
            label,
            alias,
        }
    }
}

/// Field value deprecated metadata generated from `#[deprecated(...)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DeprecatedValue {
    pub field: &'static str,
    pub value: u128,
    pub label: &'static str,
    pub since: &'static str,
    pub note: &'static str,
}

impl DeprecatedValue {
    #[must_use]
    pub const fn new(
        field: &'static str,
        value: u128,
        label: &'static str,
        since: &'static str,
        note: &'static str,
    ) -> Self {
        Self {
            field,
            value,
            label,
            since,
            note,
        }
    }
}

/// Cross-field relation kind generated from `#[requires(...)]` / `#[conflicts(...)]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldRelationKind {
    Requires,
    Conflicts,
}

/// Cross-field semantic relation. Labels are kept symbolic so this metadata can
/// be consumed by tools even when the target field is declared later.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldRelation {
    pub kind: FieldRelationKind,
    pub field: &'static str,
    pub when_label: &'static str,
    pub other_field: &'static str,
    pub other_label: &'static str,
}

impl FieldRelation {
    #[must_use]
    pub const fn requires(
        field: &'static str,
        when_label: &'static str,
        other_field: &'static str,
        other_label: &'static str,
    ) -> Self {
        Self {
            kind: FieldRelationKind::Requires,
            field,
            when_label,
            other_field,
            other_label,
        }
    }

    #[must_use]
    pub const fn conflicts(
        field: &'static str,
        left_label: &'static str,
        right_label: &'static str,
    ) -> Self {
        Self {
            kind: FieldRelationKind::Conflicts,
            field,
            when_label: left_label,
            other_field: field,
            other_label: right_label,
        }
    }
}

/// Field role for generated dispatch metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DispatchHint {
    pub field: &'static str,
    pub role: &'static str,
}

impl DispatchHint {
    #[must_use]
    pub const fn new(field: &'static str, role: &'static str) -> Self {
        Self { field, role }
    }
}

/// A generated packed integer wrapper.
///
/// Implementations are produced by `#[bitpack(storage = ...)]`.
pub trait PackedWord: Copy + Eq + core::fmt::Debug + 'static {
    /// Raw storage type, for example `u8`, `u16`, or `u64`.
    type Raw: Copy + Eq + core::fmt::Debug + 'static;

    /// Number of bits in the raw storage type.
    const STORAGE_BITS: u32;

    /// Number of bits actually consumed by fields.
    const USED_BITS: u32;

    /// Static schema for field lookup, validation, documentation, and tools.
    const FIELDS: &'static [BitFieldSpec];

    /// Raw value converted into `u128` for uniform tooling.
    #[must_use]
    fn raw_u128(self) -> u128;

    /// Raw native storage value.
    #[must_use]
    fn raw_storage(self) -> Self::Raw;

    /// Truncating constructor. Bits outside the generated layout are masked off.
    #[must_use]
    fn from_raw_u128_truncating(raw: u128) -> Self;

    /// Strict constructor. Returns `None` when `raw` has bits outside the layout.
    #[must_use]
    fn try_from_raw_u128(raw: u128) -> Option<Self>;

    /// Strict constructor with a structured error explaining why validation failed.
    fn try_from_raw_u128_checked(raw: u128) -> Result<Self, BitpackError>;

    /// Looks up field metadata by name.
    #[must_use]
    fn field(name: &str) -> Option<BitFieldSpec> {
        Self::FIELDS
            .iter()
            .copied()
            .find(|field| field.name == name)
    }

    /// Looks up field metadata by stable field id.
    #[must_use]
    fn field_by_id(id: u16) -> Option<BitFieldSpec> {
        Self::FIELDS.iter().copied().find(|field| field.id == id)
    }

    /// Reads a field by name.
    #[must_use]
    fn read_field(self, name: &str) -> Option<u128> {
        let field = Self::field(name)?;
        Some((self.raw_u128() & field.mask) >> field.shift)
    }

    /// Reads a field by stable field id.
    #[must_use]
    fn read_field_id(self, id: u16) -> Option<u128> {
        let field = Self::field_by_id(id)?;
        Some((self.raw_u128() & field.mask) >> field.shift)
    }

    /// True when the packed type contains a field with this name.
    #[must_use]
    fn has_field(name: &str) -> bool {
        Self::field(name).is_some()
    }

    /// Raw mask covering all used bits.
    #[must_use]
    fn raw_mask_u128() -> u128 {
        if Self::USED_BITS == 128 {
            u128::MAX
        } else {
            (1u128 << Self::USED_BITS) - 1
        }
    }

    /// True when the raw value contains no bits outside the declared layout.
    #[must_use]
    fn is_valid_raw_u128(raw: u128) -> bool {
        (raw & !Self::raw_mask_u128()) == 0
    }
}

/// Utility used by generated code and tests.
#[must_use]
pub const fn value_mask(width: u32) -> u128 {
    if width == 128 {
        u128::MAX
    } else {
        (1u128 << width) - 1
    }
}
