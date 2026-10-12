//! Executable action handlers for packed dispatch entries.

use big_bad_bool_core::PackedWord;

use crate::{Action, DenseDispatchEntry};
use crate::flow::{FlowError, PackedEnvelope};
use crate::passthrough::FlowDecision;

/// Trait form for callers that want a concrete handler type.
pub trait ActionHandler<T: PackedWord, Ctx> {
    type Output;
    type Error;

    fn handle(&self, word: T, action: &Action, ctx: &mut Ctx) -> Result<Self::Output, Self::Error>;
}

/// Error returned by the closure-based handler registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandlerError {
    NoAction,
    NoHandler {
        action_code: u16,
        action_name: &'static str,
    },
    PassthroughUnhandled,
    HandlerFailed(String),
}

impl core::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoAction => write!(f, "no action was attached to the packed envelope"),
            Self::NoHandler {
                action_code,
                action_name,
            } => write!(f, "no handler registered for action {action_code}:{action_name}"),
            Self::PassthroughUnhandled => write!(f, "passthrough dispatch state had no handler"),
            Self::HandlerFailed(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for HandlerError {}

impl From<HandlerError> for FlowError {
    fn from(value: HandlerError) -> Self {
        Self::Handler(value.to_string())
    }
}

type ActionFn<T, Ctx> = dyn Fn(T, &Action, &mut Ctx) -> Result<(), HandlerError>;
type PassthroughFn<T, Ctx> = dyn Fn(T, &DenseDispatchEntry, &mut Ctx) -> Result<(), HandlerError>;

struct RegisteredHandler<T: PackedWord, Ctx> {
    code: Option<u16>,
    name: Option<&'static str>,
    handle: Box<ActionFn<T, Ctx>>,
}

/// Small closure registry that turns matched actions into executable behavior.
pub struct HandlerRegistry<T: PackedWord, Ctx> {
    handlers: Vec<RegisteredHandler<T, Ctx>>,
    passthrough: Option<Box<PassthroughFn<T, Ctx>>>,
}

impl<T: PackedWord, Ctx> HandlerRegistry<T, Ctx> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
            passthrough: None,
        }
    }

    #[must_use]
    pub fn on_action(
        mut self,
        code: u16,
        handle: impl Fn(T, &Action, &mut Ctx) -> Result<(), HandlerError> + 'static,
    ) -> Self {
        self.handlers.push(RegisteredHandler {
            code: Some(code),
            name: None,
            handle: Box::new(handle),
        });
        self
    }

    #[must_use]
    pub fn on_action_name(
        mut self,
        name: &'static str,
        handle: impl Fn(T, &Action, &mut Ctx) -> Result<(), HandlerError> + 'static,
    ) -> Self {
        self.handlers.push(RegisteredHandler {
            code: None,
            name: Some(name),
            handle: Box::new(handle),
        });
        self
    }

    #[must_use]
    pub fn on_passthrough(
        mut self,
        handle: impl Fn(T, &DenseDispatchEntry, &mut Ctx) -> Result<(), HandlerError> + 'static,
    ) -> Self {
        self.passthrough = Some(Box::new(handle));
        self
    }

    pub fn handle_action(&self, word: T, action: &Action, ctx: &mut Ctx) -> Result<(), HandlerError> {
        for registered in &self.handlers {
            let code_matches = registered.code.is_some_and(|code| code == action.code);
            let name_matches = registered.name.is_some_and(|name| name == action.name);
            if code_matches || name_matches {
                return (registered.handle)(word, action, ctx);
            }
        }

        Err(HandlerError::NoHandler {
            action_code: action.code,
            action_name: action.name,
        })
    }

    pub fn handle_envelope(
        &self,
        envelope: &PackedEnvelope<T>,
        ctx: &mut Ctx,
    ) -> Result<(), HandlerError> {
        if let Some(action) = &envelope.action {
            self.handle_action(envelope.word, action, ctx)
        } else if let (Some(entry), Some(passthrough)) = (&envelope.dispatch, &self.passthrough) {
            passthrough(envelope.word, entry, ctx)
        } else if envelope.dispatch.is_some() {
            Err(HandlerError::PassthroughUnhandled)
        } else {
            Err(HandlerError::NoAction)
        }
    }

    pub fn handle_decision(&self, decision: &FlowDecision<T>, ctx: &mut Ctx) -> Result<(), HandlerError> {
        match decision {
            FlowDecision::Handled { word, action, .. } => self.handle_action(*word, action, ctx),
            FlowDecision::Passthrough { word, entry, .. } => {
                if let Some(passthrough) = &self.passthrough {
                    passthrough(*word, entry, ctx)
                } else {
                    Err(HandlerError::PassthroughUnhandled)
                }
            }
            FlowDecision::Rejected { .. } => Err(HandlerError::PassthroughUnhandled),
        }
    }
}

impl<T: PackedWord, Ctx> Default for HandlerRegistry<T, Ctx> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: PackedWord, Ctx: 'static> crate::flow::PackedFlow<T, Ctx> {
    /// Adds a final execution step using a handler registry.
    #[must_use]
    pub fn handle_with_registry(self, registry: HandlerRegistry<T, Ctx>) -> Self {
        self.and_then("handle-with-registry", move |envelope, ctx| {
            registry.handle_envelope(&envelope, ctx)?;
            Ok(envelope)
        })
    }
}
