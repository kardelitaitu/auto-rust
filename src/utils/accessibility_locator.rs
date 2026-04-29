//! Accessibility locator parser for task selector inputs.
//!
//! This module only handles grammar parsing and validation.
//! Resolution against DOM/CDP is handled by separate runtime code.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorMatchMode {
    Exact,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessibilityLocator {
    pub role: String,
    pub name: String,
    pub scope: Option<String>,
    pub match_mode: LocatorMatchMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSelector {
    Css(String),
    Accessibility(AccessibilityLocator),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocatorParseError {
    EmptyInput,
    InvalidRole,
    MissingRequiredField(&'static str),
    MalformedSegment(String),
    DuplicateField(&'static str),
    UnsupportedMatchMode(String),
    QuoteStyleNotSupported(&'static str),
}

impl fmt::Display for LocatorParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "selector is empty"),
            Self::InvalidRole => write!(f, "role value is invalid"),
            Self::MissingRequiredField(field) => write!(f, "missing required field: {field}"),
            Self::MalformedSegment(seg) => write!(f, "malformed locator segment: {seg}"),
            Self::DuplicateField(field) => write!(f, "duplicate locator field: {field}"),
            Self::UnsupportedMatchMode(mode) => write!(f, "unsupported match mode: {mode}"),
            Self::QuoteStyleNotSupported(field) => {
                write!(f, "field '{field}' must use single quotes in v1")
            }
        }
    }
}

impl std::error::Error for LocatorParseError {}

/// Parse selector input into either CSS selector or accessibility locator.
///
/// Rules:
/// - Non `role=` strings are treated as CSS selectors.
/// - `role=` strings must follow:
///   `role=<role>[name='...'][scope='...'][match=exact|contains]`
/// - Malformed `role=` strings return parse errors (never CSS fallback).
pub fn parse_selector_input(input: &str) -> Result<ParsedSelector, LocatorParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(LocatorParseError::EmptyInput);
    }

    if !trimmed.starts_with("role=") {
        return Ok(ParsedSelector::Css(trimmed.to_string()));
    }

    let rest = &trimmed["role=".len()..];
    let role_end = rest.find('[').unwrap_or(rest.len());
    let role = rest[..role_end].trim();
    if !is_valid_role(role) {
        return Err(LocatorParseError::InvalidRole);
    }

    let mut name: Option<String> = None;
    let mut scope: Option<String> = None;
    let mut match_mode = LocatorMatchMode::Exact;

    let mut i = role_end;
    while i < rest.len() {
        if !rest[i..].starts_with('[') {
            return Err(LocatorParseError::MalformedSegment(rest[i..].to_string()));
        }
        let close_rel = rest[i + 1..]
            .find(']')
            .ok_or_else(|| LocatorParseError::MalformedSegment(rest[i..].to_string()))?;
        let close = i + 1 + close_rel;
        let segment = rest[i + 1..close].trim();
        parse_segment(segment, &mut name, &mut scope, &mut match_mode)?;
        i = close + 1;
    }

    let name = name.ok_or(LocatorParseError::MissingRequiredField("name"))?;
    Ok(ParsedSelector::Accessibility(AccessibilityLocator {
        role: role.to_string(),
        name,
        scope,
        match_mode,
    }))
}

fn parse_segment(
    segment: &str,
    name: &mut Option<String>,
    scope: &mut Option<String>,
    match_mode: &mut LocatorMatchMode,
) -> Result<(), LocatorParseError> {
    let mut split = segment.splitn(2, '=');
    let key = split.next().unwrap_or("").trim();
    let value = split
        .next()
        .ok_or_else(|| LocatorParseError::MalformedSegment(segment.to_string()))?
        .trim();

    match key {
        "name" => {
            if name.is_some() {
                return Err(LocatorParseError::DuplicateField("name"));
            }
            if value.starts_with('"') && value.ends_with('"') {
                return Err(LocatorParseError::QuoteStyleNotSupported("name"));
            }
            *name = Some(parse_single_quoted(value, "name")?);
        }
        "scope" => {
            if scope.is_some() {
                return Err(LocatorParseError::DuplicateField("scope"));
            }
            if value.starts_with('"') && value.ends_with('"') {
                return Err(LocatorParseError::QuoteStyleNotSupported("scope"));
            }
            *scope = Some(parse_single_quoted(value, "scope")?);
        }
        "match" => match value {
            "exact" => *match_mode = LocatorMatchMode::Exact,
            "contains" => *match_mode = LocatorMatchMode::Contains,
            other => return Err(LocatorParseError::UnsupportedMatchMode(other.to_string())),
        },
        _ => {
            return Err(LocatorParseError::MalformedSegment(segment.to_string()));
        }
    }
    Ok(())
}

fn parse_single_quoted(value: &str, field: &'static str) -> Result<String, LocatorParseError> {
    if !(value.starts_with('\'') && value.ends_with('\'')) || value.len() < 2 {
        return Err(LocatorParseError::MalformedSegment(format!(
            "{field} must be single-quoted"
        )));
    }
    Ok(value[1..value.len() - 1].to_string())
}

fn is_valid_role(role: &str) -> bool {
    if role.is_empty() {
        return false;
    }
    role.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_css_selector_as_css_variant() {
        let parsed = parse_selector_input("button[aria-label='Like']").unwrap();
        assert_eq!(
            parsed,
            ParsedSelector::Css("button[aria-label='Like']".to_string())
        );
    }

    #[test]
    fn parses_locator_exact_defaults() {
        let parsed = parse_selector_input("role=button[name='Save changes']").unwrap();
        assert_eq!(
            parsed,
            ParsedSelector::Accessibility(AccessibilityLocator {
                role: "button".to_string(),
                name: "Save changes".to_string(),
                scope: None,
                match_mode: LocatorMatchMode::Exact,
            })
        );
    }

    #[test]
    fn parses_locator_with_scope_and_contains() {
        let parsed =
            parse_selector_input("role=link[name='Profile'][scope='main'][match=contains]")
                .unwrap();
        assert_eq!(
            parsed,
            ParsedSelector::Accessibility(AccessibilityLocator {
                role: "link".to_string(),
                name: "Profile".to_string(),
                scope: Some("main".to_string()),
                match_mode: LocatorMatchMode::Contains,
            })
        );
    }

    #[test]
    fn fails_when_name_missing() {
        let err = parse_selector_input("role=button").unwrap_err();
        assert_eq!(err, LocatorParseError::MissingRequiredField("name"));
    }

    #[test]
    fn fails_for_double_quote_name_in_v1() {
        let err = parse_selector_input("role=button[name=\"Save\"]").unwrap_err();
        assert_eq!(err, LocatorParseError::QuoteStyleNotSupported("name"));
    }

    #[test]
    fn fails_for_unsupported_match_mode() {
        let err = parse_selector_input("role=button[name='Save'][match=prefix]").unwrap_err();
        assert_eq!(
            err,
            LocatorParseError::UnsupportedMatchMode("prefix".to_string())
        );
    }

    #[test]
    fn fails_for_duplicate_name_field() {
        let err =
            parse_selector_input("role=button[name='Save'][name='Again']").unwrap_err();
        assert_eq!(err, LocatorParseError::DuplicateField("name"));
    }

    #[test]
    fn fails_for_empty_input() {
        let err = parse_selector_input("   ").unwrap_err();
        assert_eq!(err, LocatorParseError::EmptyInput);
    }

    #[test]
    fn fails_for_invalid_role_token() {
        let err = parse_selector_input("role=Button[name='Save']").unwrap_err();
        assert_eq!(err, LocatorParseError::InvalidRole);
    }

    #[test]
    fn css_compat_regression_matrix() {
        let selectors = [
            "#submit",
            ".btn.primary",
            "button[aria-label='Like']",
            "[role='button'][aria-label='Follow @user']",
            "[data-testid='tweetButtonInline']",
            "main div:nth-child(2) > button",
            "input[name='q']",
            "a[href*='/status/']",
            "div[class*='tweet'] button:nth-of-type(1)",
            "div[role='button']",
            "role[name='button']",
            "[aria-describedby='hint'][tabindex='0']",
        ];

        for selector in selectors {
            let parsed = parse_selector_input(selector).unwrap();
            assert_eq!(parsed, ParsedSelector::Css(selector.to_string()));
        }
    }

    #[test]
    fn css_compat_trims_whitespace_but_preserves_selector() {
        let parsed = parse_selector_input("   [data-testid='like']   ").unwrap();
        assert_eq!(
            parsed,
            ParsedSelector::Css("[data-testid='like']".to_string())
        );
    }
}
