//! Parsing of `#@` plugin annotations from Dolfin comments.
//!
//! Annotations are regular comments whose text begins with `@`. Any parser
//! that does not recognise an annotation simply ignores the comment: the
//! core language is unaffected.
//!
//! # Syntax
//!
//! ```text
//! #@ sparnatural
//! concept Person:
//!   has name: string
//!
//!   #@ sparnatural(widget=list)
//!   has employer: Organization
//!
//!   #@ sparnatural(widget=date_range, label="Birth date")
//!   has birthDate: date
//! ```

use crate::comment::Comment;

/// A structured annotation parsed from a `#@` comment.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedAnnotation {
    /// The annotation name (e.g. `"sparnatural"`).
    pub name: String,
    /// Key-value arguments (e.g. `[("widget", "list")]`).
    /// Empty when the annotation carries no arguments.
    pub args: Vec<(String, String)>,
}

impl ParsedAnnotation {
    /// Look up the value of an argument by key.
    pub fn arg(&self, key: &str) -> Option<&str> {
        self.args
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    /// Returns `true` when the annotation has no arguments.
    pub fn is_bare(&self) -> bool {
        self.args.is_empty()
    }
}

/// Try to parse a `#@` comment into a [`ParsedAnnotation`].
///
/// Returns `None` if the comment is not a `#@` annotation (i.e. its trimmed
/// text does not start with `@`).
///
/// The `Comment.text` field contains everything after the leading `#`, so a
/// comment written as `#@ sparnatural(widget=list)` will have
/// `text == "@ sparnatural(widget=list)"`.
pub fn parse_annotation(comment: &Comment) -> Option<ParsedAnnotation> {
    let text = comment.text.trim();
    let rest = text.strip_prefix('@')?;
    let rest = rest.trim_start();

    if rest.is_empty() {
        return None;
    }

    Some(parse_annotation_text(rest))
}

/// Parse the portion of a `#@` comment that follows the leading `@`.
///
/// Handles both bare form (`sparnatural`) and argument form
/// (`sparnatural(widget=list, label="Foo")`).
fn parse_annotation_text(text: &str) -> ParsedAnnotation {
    if let Some(paren_pos) = text.find(':') {
        let name = text[..paren_pos].trim().to_string();
        let args_text = text[paren_pos + 1..].trim_end_matches('\n').trim();
        let args = parse_args(args_text);
        ParsedAnnotation { name, args }
    } else if let Some(paren_pos) = text.find('\n') {
      let name = text[..paren_pos].trim().to_string();
        let args_text = text[paren_pos + 1..].trim_end_matches('\n').trim();
        let args = parse_args(args_text);
        ParsedAnnotation { name, args }
    } else {
        ParsedAnnotation {
            name: text.trim().to_string(),
            args: vec![],
        }
    }
}

/// Parse a comma-separated list of `key=value` pairs.
///
/// Values may optionally be surrounded by double quotes, which are stripped.
fn parse_args(text: &str) -> Vec<(String, String)> {
    if text.is_empty() {
        return vec![];
    }

    text.split('\n')
        .filter_map(|pair| {
            let pair = pair.trim();
            if pair.is_empty() {
                return None;
            }
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].trim_start_matches('@').trim().to_string();
                let val = pair[eq_pos + 1..].trim().trim_matches('"').to_string();
                Some((key, val))
            } else {
                // Bare flag without a value, treat value as "true"
                Some((
                    pair.trim_matches('@').trim().to_string(),
                    "true".to_string(),
                ))
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::Comment;
    use crate::error::{Location, Span};

    fn make_comment(text: &str) -> Comment {
        Comment {
            text: text.to_string(),
            raw: format!("#{}", text),
            span: Span {
                start: Location::default(),
                end: Location::default(),
            },
            line: 1,
            column: 1,
        }
    }

    #[test]
    fn test_bare_annotation() {
        let c = make_comment("@ sparnatural");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.name, "sparnatural");
        assert!(ann.args.is_empty());
    }

    #[test]
    fn test_annotation_with_args() {
        let c = make_comment("@ sparnatural:\n@   widget=list\n");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.name, "sparnatural");
        assert_eq!(ann.arg("widget"), Some("list"));
    }

    #[test]
    fn test_annotation_with_quoted_value() {
        let c = make_comment("@ sparnatural:\n@   widget = date_range\n@   label = \"Birth date\"");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.name, "sparnatural");
        assert_eq!(ann.arg("widget"), Some("date_range"));
        assert_eq!(ann.arg("label"), Some("Birth date"));
    }

    #[test]
    fn test_not_an_annotation() {
        let c = make_comment(" regular comment");
        assert!(parse_annotation(&c).is_none());
    }

    #[test]
    fn test_bare_flag_arg() {
        let c = make_comment("@ sparnatural:\n@   searchable\n");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.arg("searchable"), Some("true"));
    }
}
