//! FP-style packed value flow chains.
//!
//! This is deliberately practical FP: each step receives an envelope and either
//! returns the next envelope or a structured flow error.

use big_bad_bool_core::{BitpackError, PackedWord};

use crate::{
    decode_fields, decide_passthrough, patch_word, Action, DecodedFieldValue, DenseDispatchEntry,
    DenseDispatchTable, FieldValue, PassthroughMode, ReconstructionError, RejectReason,
};

/// Runtime envelope carried through a packed operation chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackedEnvelope<T: PackedWord> {
    pub word: T,
    pub raw: u128,
    pub fields: Vec<DecodedFieldValue>,
    pub dispatch: Option<DenseDispatchEntry>,
    pub action: Option<Action>,
    pub notes: Vec<String>,
}

impl<T: PackedWord> PackedEnvelope<T> {
    #[must_use]
    pub fn new(word: T) -> Self {
        Self {
            raw: word.raw_u128(),
            fields: decode_fields(word),
            word,
            dispatch: None,
            action: None,
            notes: Vec::new(),
        }
    }

    pub fn from_raw_checked(raw: u128) -> Result<Self, FlowError> {
        T::try_from_raw_u128_checked(raw)
            .map(Self::new)
            .map_err(FlowError::Decode)
    }

    #[must_use]
    pub fn refresh(mut self) -> Self {
        self.raw = self.word.raw_u128();
        self.fields = decode_fields(self.word);
        self
    }

    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Failure while running a packed flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowError {
    Decode(BitpackError),
    Reconstruction(ReconstructionError),
    MissingDispatch,
    Rejected {
        raw: u128,
        entry: DenseDispatchEntry,
        reason: RejectReason,
    },
    Handler(String),
    Custom(String),
}

impl core::fmt::Display for FlowError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Decode(error) => write!(f, "decode failed: {error}"),
            Self::Reconstruction(error) => write!(f, "reconstruction failed: {error}"),
            Self::MissingDispatch => write!(f, "flow expected a dispatch entry, but none was present"),
            Self::Rejected { raw, reason, .. } => {
                write!(f, "packed flow rejected raw value {raw} with reason {reason:?}")
            }
            Self::Handler(message) => write!(f, "handler failed: {message}"),
            Self::Custom(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for FlowError {}

impl From<ReconstructionError> for FlowError {
    fn from(value: ReconstructionError) -> Self {
        Self::Reconstruction(value)
    }
}

/// A named flow step.
pub struct PackedStep<T: PackedWord, Ctx> {
    pub name: &'static str,
    apply: Box<dyn Fn(PackedEnvelope<T>, &mut Ctx) -> Result<PackedEnvelope<T>, FlowError>>,
}

impl<T: PackedWord, Ctx> PackedStep<T, Ctx> {
    #[must_use]
    pub fn new(
        name: &'static str,
        apply: impl Fn(PackedEnvelope<T>, &mut Ctx) -> Result<PackedEnvelope<T>, FlowError> + 'static,
    ) -> Self {
        Self {
            name,
            apply: Box::new(apply),
        }
    }
}

/// Ordered packed-value operation chain.
pub struct PackedFlow<T: PackedWord, Ctx> {
    name: &'static str,
    steps: Vec<PackedStep<T, Ctx>>,
}

impl<T: PackedWord, Ctx> PackedFlow<T, Ctx> {
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    #[must_use]
    pub fn step(mut self, step: PackedStep<T, Ctx>) -> Self {
        self.steps.push(step);
        self
    }

    #[must_use]
    pub fn and_then(
        self,
        name: &'static str,
        apply: impl Fn(PackedEnvelope<T>, &mut Ctx) -> Result<PackedEnvelope<T>, FlowError> + 'static,
    ) -> Self {
        self.step(PackedStep::new(name, apply))
    }

    #[must_use]
    pub fn tap(
        self,
        name: &'static str,
        inspect: impl Fn(&PackedEnvelope<T>, &mut Ctx) -> Result<(), FlowError> + 'static,
    ) -> Self {
        self.and_then(name, move |envelope, ctx| {
            inspect(&envelope, ctx)?;
            Ok(envelope)
        })
    }

    #[must_use]
    pub fn patch_field(self, field: &'static str, value: u128) -> Self {
        self.and_then("patch-field", move |mut envelope, _ctx| {
            envelope.word = patch_word(envelope.word, &[FieldValue::new(field, value)])?;
            envelope.dispatch = None;
            envelope.action = None;
            Ok(envelope.refresh())
        })
    }

    #[must_use]
    pub fn patch_fields(self, fields: &'static [FieldValue]) -> Self {
        self.and_then("patch-fields", move |mut envelope, _ctx| {
            envelope.word = patch_word(envelope.word, fields)?;
            envelope.dispatch = None;
            envelope.action = None;
            Ok(envelope.refresh())
        })
    }

    #[must_use]
    pub fn dispatch_dense(self, table: DenseDispatchTable<T>) -> Self {
        self.and_then("dispatch-dense", move |mut envelope, _ctx| {
            envelope.dispatch = table.dispatch(envelope.word).cloned();
            envelope.action = match &envelope.dispatch {
                Some(DenseDispatchEntry::Matched { action, .. }) => Some(action.clone()),
                _ => None,
            };
            Ok(envelope)
        })
    }

    #[must_use]
    pub fn passthrough(self, mode: PassthroughMode) -> Self {
        self.and_then("passthrough", move |mut envelope, _ctx| {
            let entry = envelope.dispatch.clone().ok_or(FlowError::MissingDispatch)?;
            match decide_passthrough(envelope.word, entry, mode) {
                crate::passthrough::FlowDecision::Handled { entry, action, .. } => {
                    envelope.dispatch = Some(entry);
                    envelope.action = Some(action);
                    Ok(envelope)
                }
                crate::passthrough::FlowDecision::Passthrough { entry, reason, .. } => {
                    envelope.dispatch = Some(entry);
                    envelope.action = None;
                    Ok(envelope.with_note(format!("passthrough:{reason:?}")))
                }
                crate::passthrough::FlowDecision::Rejected { entry, reason, .. } => Err(FlowError::Rejected {
                    raw: envelope.raw,
                    entry,
                    reason,
                }),
            }
        })
    }

    pub fn run(&self, envelope: PackedEnvelope<T>, ctx: &mut Ctx) -> Result<PackedEnvelope<T>, FlowError> {
        let mut current = envelope;
        for step in &self.steps {
            current = (step.apply)(current, ctx)?;
        }

        Ok(current)
    }

    pub fn run_word(&self, word: T, ctx: &mut Ctx) -> Result<PackedEnvelope<T>, FlowError> {
        self.run(PackedEnvelope::new(word), ctx)
    }

    pub fn run_raw_checked(&self, raw: u128, ctx: &mut Ctx) -> Result<PackedEnvelope<T>, FlowError> {
        self.run(PackedEnvelope::from_raw_checked(raw)?, ctx)
    }
}
