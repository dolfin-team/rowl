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

    /// Look up the value of an argument by key, if not found then tries the other keys of the list one by one.
    /// Stop at the first key that return a value, or return None if none are found.
    pub fn arg_with_alt(&self, keys: &[&str]) -> Option<&str> {
      keys.iter().find_map(|key| self.arg(key))
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
    if let Some(nl_pos) = text.find('\n') {
        // Multi-line form — name ends at first newline (strip trailing ':' if present).
        // Check newline before ':' so IRIs in values (e.g. <http://...>) don't corrupt the name.
        let name = text[..nl_pos].trim().trim_end_matches(':').trim().to_string();
        let args_text = text[nl_pos + 1..].trim_end_matches('\n').trim();
        let args = parse_args(args_text);
        ParsedAnnotation { name, args }
    } else if let Some(colon_pos) = text.find(':') {
        // Single-line colon form: `sparnatural:` header with no args.
        let name = text[..colon_pos].trim().to_string();
        let args_text = text[colon_pos + 1..].trim_end_matches('\n').trim();
        let args = parse_args(args_text);
        ParsedAnnotation { name, args }
    } else {
        ParsedAnnotation {
            name: text.trim().to_string(),
            args: vec![],
        }
    }
}

/// Parse a newline-separated list of `key=value` (or `key value`) pairs.
///
/// Values may optionally be surrounded by double quotes, which are stripped.
///
/// A line with strictly more indentation (spaces after `@`) than the previous
/// key-level line is treated as a continuation of that key's value and joined
/// with a space.  A trailing `\` is stripped from any line if present, but is
/// not required to trigger continuation — indentation alone is sufficient.
fn parse_args(text: &str) -> Vec<(String, String)> {
    if text.is_empty() {
        return vec![];
    }

    // First pass: join continuation lines by relative indentation.
    let mut joined: Vec<(String, usize)> = Vec::new(); // (content, key-level indent)
    for raw in text.split('\n') {
        let after_at = raw.trim_start_matches('@');
        let trimmed = after_at.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        let indent = after_at.len() - trimmed.len();
        let content = trimmed.strip_suffix('\\').unwrap_or(trimmed).trim_end();
        if let Some((last_content, last_indent)) = joined.last_mut() {
            if indent > *last_indent {
                last_content.push(' ');
                last_content.push_str(content);
                continue;
            }
        }
        joined.push((content.to_string(), indent));
    }

    // Second pass: parse each joined line as key=value or key value.
    joined
        .iter()
        .filter_map(|(line, _)| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim().to_string();
                let val = line[eq_pos + 1..].trim().trim_matches('"').to_string();
                Some((key, val))
            } else if let Some((k, v)) = line.split_once(char::is_whitespace) {
                Some((k.trim().to_string(), v.trim().trim_matches('"').to_string()))
            } else {
                Some((line.to_string(), "true".to_string()))
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

    #[test]
    fn test_space_separated_arg() {
        let c = make_comment("@ sparnatural:\n@   widget list\n");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.arg("widget"), Some("list"));
    }

    #[test]
    fn test_space_separated_quoted_arg() {
        let c = make_comment("@ sparnatural:\n@   label \"Birth date\"\n");
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.arg("label"), Some("Birth date"));
    }

    /// Multiline value continuation via indentation alone (no `\` required).
    ///
    /// Merged comment from:
    ///   #@glossary
    ///   #@ definition Any individual
    ///   #@   of the species
    ///   #@   Homo sapiens
    ///   #@ label Human being
    ///   #@ altLabel Someone
    ///
    /// Continuation lines have more indentation after `#@` than the key line.
    ///
    /// Must produce the same result as:
    ///   #@glossary
    ///   #@  definition Any individual of the species Homo sapiens
    ///   #@  label Human being
    ///   #@  altLabel Someone
    #[test]
    fn test_multiline_value_continuation() {
        let c = make_comment(
            "@glossary\n@ definition Any individual\n@   of the species\n@   Homo sapiens\n@ label Human being\n@ altLabel Someone",
        );
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.name, "glossary");
        assert_eq!(
            ann.arg("definition"),
            Some("Any individual of the species Homo sapiens")
        );
        assert_eq!(ann.arg("label"), Some("Human being"));
        assert_eq!(ann.arg("altLabel"), Some("Someone"));
    }

    /// Optional `\` at end of continuation lines is stripped but not required.
    #[test]
    fn test_multiline_value_continuation_optional_backslash() {
        let c = make_comment(
            "@glossary\n@ definition Any individual\\\n@   of the species\\\n@   Homo sapiens\n@ label Human being\n@ altLabel Someone",
        );
        let ann = parse_annotation(&c).unwrap();
        assert_eq!(ann.name, "glossary");
        assert_eq!(
            ann.arg("definition"),
            Some("Any individual of the species Homo sapiens")
        );
    }
}
