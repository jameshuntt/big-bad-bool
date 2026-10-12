//! Policy-aware passthrough dispatch decisions.
//!
//! Dense dispatch tells us how a packed value is classified. This module decides
//! whether non-matched states should reject immediately or continue to another
//! layer/runtime.

use big_bad_bool_core::PackedWord;

use crate::{Action, DenseDispatchEntry, DenseDispatchTable};

/// Controls which non-matched dispatch states are allowed to continue down-chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassthroughMode {
    /// Only exactly matched states are handled. Everything else rejects.
    RejectAllNonMatched,
    /// Forward normal states that have no matching rule.
    ForwardUnmatched,
    /// Forward reserved states only.
    ForwardReserved,
    /// Forward invalid states only.
    ForwardInvalid,
    /// Forward overlap states only.
    ForwardOverlaps,
    /// Forward policy/rule conflicts only.
    ForwardPolicyConflicts,
    /// Forward unmatched/reserved/invalid/overlap/conflict states.
    ForwardEverything,
}

impl PassthroughMode {
    #[must_use]
    pub const fn allows(self, reason: PassthroughReason) -> bool {
        match self {
            Self::RejectAllNonMatched => false,
            Self::ForwardUnmatched => matches!(reason, PassthroughReason::Unmatched),
            Self::ForwardReserved => matches!(reason, PassthroughReason::Reserved),
            Self::ForwardInvalid => matches!(reason, PassthroughReason::Invalid),
            Self::ForwardOverlaps => matches!(reason, PassthroughReason::Overlap),
            Self::ForwardPolicyConflicts => matches!(reason, PassthroughReason::PolicyRuleConflict),
            Self::ForwardEverything => true,
        }
    }
}

/// Reason a state was forwarded instead of handled locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassthroughReason {
    Unmatched,
    Reserved,
    Invalid,
    Overlap,
    PolicyRuleConflict,
}

/// Reason a state was rejected by a passthrough policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RejectReason {
    Unmatched,
    Reserved,
    Invalid,
    Overlap,
    PolicyRuleConflict,
    MissingDispatchEntry,
}

impl From<PassthroughReason> for RejectReason {
    fn from(value: PassthroughReason) -> Self {
        match value {
            PassthroughReason::Unmatched => Self::Unmatched,
            PassthroughReason::Reserved => Self::Reserved,
            PassthroughReason::Invalid => Self::Invalid,
            PassthroughReason::Overlap => Self::Overlap,
            PassthroughReason::PolicyRuleConflict => Self::PolicyRuleConflict,
        }
    }
}

/// Decision produced after applying passthrough rules to a dispatch entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlowDecision<T: PackedWord> {
    Handled {
        word: T,
        entry: DenseDispatchEntry,
        action: Action,
    },
    Passthrough {
        word: T,
        entry: DenseDispatchEntry,
        reason: PassthroughReason,
    },
    Rejected {
        word: T,
        entry: DenseDispatchEntry,
        reason: RejectReason,
    },
}

impl<T: PackedWord> FlowDecision<T> {
    #[must_use]
    pub fn word(&self) -> T {
        match self {
            Self::Handled { word, .. }
            | Self::Passthrough { word, .. }
            | Self::Rejected { word, .. } => *word,
        }
    }

    #[must_use]
    pub fn is_handled(&self) -> bool {
        matches!(self, Self::Handled { .. })
    }

    #[must_use]
    pub fn is_passthrough(&self) -> bool {
        matches!(self, Self::Passthrough { .. })
    }

    #[must_use]
    pub fn is_rejected(&self) -> bool {
        matches!(self, Self::Rejected { .. })
    }
}

impl<T: PackedWord> DenseDispatchTable<T> {
    /// Dispatches a word and applies a passthrough policy to non-matched states.
    #[must_use]
    pub fn dispatch_with_passthrough(&self, word: T, mode: PassthroughMode) -> FlowDecision<T> {
        match self.dispatch(word).cloned() {
            Some(entry) => decide_passthrough(word, entry, mode),
            None => FlowDecision::Rejected {
                word,
                entry: DenseDispatchEntry::Unmatched,
                reason: RejectReason::MissingDispatchEntry,
            },
        }
    }

    /// Strictly decodes a raw value, then dispatches it with passthrough policy.
    #[must_use]
    pub fn dispatch_raw_with_passthrough(
        &self,
        raw: u128,
        mode: PassthroughMode,
    ) -> Option<FlowDecision<T>> {
        let word = T::try_from_raw_u128(raw)?;
        Some(self.dispatch_with_passthrough(word, mode))
    }
}

/// Applies a passthrough mode to one precomputed dense dispatch entry.
#[must_use]
pub fn decide_passthrough<T: PackedWord>(
    word: T,
    entry: DenseDispatchEntry,
    mode: PassthroughMode,
) -> FlowDecision<T> {
    match &entry {
        DenseDispatchEntry::Matched { action, .. } => FlowDecision::Handled {
            word,
            entry: entry.clone(),
            action: action.clone(),
        },
        DenseDispatchEntry::Unmatched => passthrough_or_reject(
            word,
            entry,
            mode,
            PassthroughReason::Unmatched,
        ),
        DenseDispatchEntry::Reserved { .. } => passthrough_or_reject(
            word,
            entry,
            mode,
            PassthroughReason::Reserved,
        ),
        DenseDispatchEntry::Invalid { .. } => passthrough_or_reject(
            word,
            entry,
            mode,
            PassthroughReason::Invalid,
        ),
        DenseDispatchEntry::Overlap { .. } => passthrough_or_reject(
            word,
            entry,
            mode,
            PassthroughReason::Overlap,
        ),
        DenseDispatchEntry::PolicyRuleConflict { .. } => passthrough_or_reject(
            word,
            entry,
            mode,
            PassthroughReason::PolicyRuleConflict,
        ),
    }
}

fn passthrough_or_reject<T: PackedWord>(
    word: T,
    entry: DenseDispatchEntry,
    mode: PassthroughMode,
    reason: PassthroughReason,
) -> FlowDecision<T> {
    if mode.allows(reason) {
        FlowDecision::Passthrough {
            word,
            entry,
            reason,
        }
    } else {
        FlowDecision::Rejected {
            word,
            entry,
            reason: reason.into(),
        }
    }
}
