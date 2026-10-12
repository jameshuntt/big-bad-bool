//! Reconstruction and field-overlay helpers for packed words.
//!
//! The macro-generated constructors are ideal when callers already know every
//! field at compile time. This module covers the builder/runtime case: callers
//! have symbolic field assignments and need to reconstruct a packed word safely.

use big_bad_bool_core::{BitFieldSpec, BitpackError, PackedWord};

/// A symbolic assignment to one generated packed field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldValue {
    pub field: &'static str,
    pub value: u128,
}

impl FieldValue {
    #[must_use]
    pub const fn new(field: &'static str, value: u128) -> Self {
        Self { field, value }
    }
}

/// A decoded field/value pair from a packed word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DecodedFieldValue {
    pub field: &'static str,
    pub value: u128,
}

/// Failure while reconstructing a packed value from symbolic assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReconstructionError {
    UnknownField {
        field: &'static str,
    },
    DuplicateField {
        field: &'static str,
    },
    ValueOutOfRange {
        field: &'static str,
        value: u128,
        max: u128,
    },
    RawRejected {
        raw: u128,
        error: BitpackError,
    },
}

impl core::fmt::Display for ReconstructionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnknownField { field } => write!(f, "unknown bitpack field `{field}`"),
            Self::DuplicateField { field } => write!(f, "duplicate bitpack field assignment `{field}`"),
            Self::ValueOutOfRange { field, value, max } => {
                write!(f, "field `{field}` value {value} exceeds max {max}")
            }
            Self::RawRejected { raw, error } => {
                write!(f, "raw value {raw} was rejected during reconstruction: {error}")
            }
        }
    }
}

impl std::error::Error for ReconstructionError {}

/// Builder for reconstructing a packed word from symbolic field assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedReconstructor<T: PackedWord> {
    raw: u128,
    _marker: core::marker::PhantomData<T>,
}

impl<T: PackedWord> PackedReconstructor<T> {
    /// Starts from an all-zero packed value.
    #[must_use]
    pub const fn zeroed() -> Self {
        Self {
            raw: 0,
            _marker: core::marker::PhantomData,
        }
    }

    /// Starts from an existing packed word.
    #[must_use]
    pub fn from_word(word: T) -> Self {
        Self {
            raw: word.raw_u128(),
            _marker: core::marker::PhantomData,
        }
    }

    /// Starts from a raw value after strict layout validation.
    pub fn from_raw_checked(raw: u128) -> Result<Self, ReconstructionError> {
        T::try_from_raw_u128_checked(raw)
            .map(Self::from_word)
            .map_err(|error| ReconstructionError::RawRejected { raw, error })
    }

    /// Returns the current raw reconstruction state.
    #[must_use]
    pub const fn raw(&self) -> u128 {
        self.raw
    }

    /// Sets one field by symbolic name.
    pub fn set_field(mut self, field: &'static str, value: u128) -> Result<Self, ReconstructionError> {
        let spec = T::field(field).ok_or(ReconstructionError::UnknownField { field })?;
        self.raw = write_field(self.raw, spec, value)?;
        Ok(self)
    }

    /// Applies assignments in order. Later values override earlier values for the same field.
    pub fn overlay_fields(mut self, fields: &[FieldValue]) -> Result<Self, ReconstructionError> {
        for assignment in fields.iter().copied() {
            self = self.set_field(assignment.field, assignment.value)?;
        }

        Ok(self)
    }

    /// Applies assignments but rejects duplicate fields.
    pub fn overlay_fields_strict(mut self, fields: &[FieldValue]) -> Result<Self, ReconstructionError> {
        for (idx, assignment) in fields.iter().copied().enumerate() {
            if fields
                .iter()
                .take(idx)
                .any(|previous| previous.field == assignment.field)
            {
                return Err(ReconstructionError::DuplicateField {
                    field: assignment.field,
                });
            }

            self = self.set_field(assignment.field, assignment.value)?;
        }

        Ok(self)
    }

    /// Builds the final packed word using strict raw validation.
    pub fn build(self) -> Result<T, ReconstructionError> {
        T::try_from_raw_u128_checked(self.raw).map_err(|error| ReconstructionError::RawRejected {
            raw: self.raw,
            error,
        })
    }

    /// Builds the final packed word by truncating to the generated layout.
    #[must_use]
    pub fn build_truncating(self) -> T {
        T::from_raw_u128_truncating(self.raw)
    }
}

/// Decodes every generated field in declaration order.
#[must_use]
pub fn decode_fields<T: PackedWord>(word: T) -> Vec<DecodedFieldValue> {
    T::FIELDS
        .iter()
        .map(|field| DecodedFieldValue {
            field: field.name,
            value: word.read_field(field.name).unwrap_or_default(),
        })
        .collect()
}

/// Reconstructs a packed word from a strict set of field assignments.
pub fn reconstruct_word<T: PackedWord>(fields: &[FieldValue]) -> Result<T, ReconstructionError> {
    PackedReconstructor::<T>::zeroed()
        .overlay_fields_strict(fields)?
        .build()
}

/// Patches an existing word with field assignments. Later assignments override earlier ones.
pub fn patch_word<T: PackedWord>(word: T, fields: &[FieldValue]) -> Result<T, ReconstructionError> {
    PackedReconstructor::<T>::from_word(word)
        .overlay_fields(fields)?
        .build()
}

fn write_field(raw: u128, field: BitFieldSpec, value: u128) -> Result<u128, ReconstructionError> {
    if value > field.max_value() {
        return Err(ReconstructionError::ValueOutOfRange {
            field: field.name,
            value,
            max: field.max_value(),
        });
    }

    let cleared = raw & !field.mask;
    Ok(cleared | ((value & field.value_mask) << field.shift))
}
