//! Decision strategy implementations.
//!
//! This module contains all decision strategy implementations:
//! - Legacy: Rule-based keyword matching
//! - Persona: Persona-weighted decisions
//! - LLM: OpenAI-based decisions
//! - Hybrid: Combined strategy approach
//! - Unified: Single-call decision + content

pub(crate) mod hybrid;
pub(crate) mod legacy;
pub(crate) mod llm;
pub(crate) mod persona;
pub(crate) mod unified;
