//! Custom lexer for Dolfin that handles indentation-based syntax.
//!
//! This lexer produces INDENT and DEDENT tokens based on indentation changes,
//! similar to Python's lexer.

use crate::{
    ast::TemporalKind,
    comment::{Comment, CommentSink},
    error::{ErrorCode, LexerError, Location, Span},
};
use logos::Logos;
use std::collections::VecDeque;
use tracing::debug;

/// Token types for the Dolfin language
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    /// 'package' keyword
    Package,
    /// 'prefix' keyword
    Prefix,
    /// '@iri_name' annotation
    IriName,
    /// 'as' keyword
    As,
    /// 'concept' keyword
    Concept,
    /// 'property' keyword
    Property,
    /// 'one of' keyword
    OneOf,
    /// 'rule' keyword
    Rule,
    /// 'match' keyword
    Match,
    /// 'then' keyword
    Then,
    /// 'sub' keyword
    Sub,
    /// 'has' keyword
    Has,
    /// 'key' keyword (marks a property as part of the primary key)
    Key,
    /// 'a' keyword (type assertion)
    Is,
    /// 'fact' keyword (instance declaration)
    Fact,
    /// 'query' keyword (query definition)
    Query,
    /// 'return' keyword (query projection block)
    Return,
    /// 'either' keyword (query disjunction)
    Either,
    /// 'or' keyword (query disjunction branch)
    Or,
    /// 'group by' compound keyword (query aggregation)
    GroupBy,
    /// 'is' keyword (inverse property assertion in fact blocks)
    IsInverse,
    /// 'inverse of' keyword pair (property axiom)
    InverseOf,
    /// 'equivalent to' keyword pair (property axiom)
    EquivalentTo,
    /// 'transitive' keyword (property characteristic)
    Transitive,
    /// 'symmetric' keyword (property characteristic)
    Symmetric,
    /// 'reflexive' keyword (property characteristic)
    Reflexive,
    /// 'unit' keyword (unit declaration)
    Unit,
    /// 'family' keyword (unit family declaration)
    Family,
    /// 'nominal' keyword (nominal/incommensurable unit declaration)
    Nominal,
    /// 'scale' keyword (unit scale factor)
    Scale,

    // Quantifier keywords
    /// 'all' quantifier
    All,
    /// 'none' quantifier
    None,
    /// 'at_least' quantifier
    AtLeast,
    /// 'at_most' quantifier
    AtMost,
    /// 'exactly' quantifier
    Exactly,
    /// 'between' quantifier
    Between,
    /// 'of' keyword (used in 'one of:' construct)
    Of,

    // Cardinality keywords
    /// 'one' cardinality
    One,
    /// 'any' cardinality
    Any,
    /// 'some' cardinality
    Some,
    /// 'optional' cardinality
    Optional,

    // Primitive type keywords
    /// 'string' type keyword
    TString,
    /// 'int' type keyword
    TInt,
    /// 'float' type keyword
    TFloat,
    /// 'boolean' type keyword
    TBoolean,
    /// 'date' type keyword
    TDate,
    /// 'date_time' type keyword
    TDateTime,
    /// 'time' type keyword
    TTime,
    /// 'duration' type keyword
    TDuration,

    // Literals
    /// Integer literal
    Int(i64),
    /// Floating-point literal
    Float(f64),
    /// String literal
    String(String),
    /// Boolean literal
    Boolean(bool),
    /// IRI literal
    Iri(String),
    /// Temporal smart literal: (declared kind, raw content between parens)
    TemporalLit((TemporalKind, String)),
    /// Physical-quantity smart literal: raw content between `quantity(` `)`.
    QuantityLit(String),
    /// `@locale <arg>` directive (raw argument)
    LocaleDirective(String),
    /// `@timezone <arg>` directive (raw argument)
    TimezoneDirective(String),

    // Identifiers and special
    /// Prefixed name (alias:LocalName with no whitespace around colon)
    PrefixedName((String, String)),
    /// Identifier name
    Name(String),
    /// Variable (starts with ?)
    Variable(String),

    // Punctuation
    /// Colon (:)
    Colon,
    /// Comma (,)
    Comma,
    /// Dot (.)
    Dot,
    /// Arrow (->)
    Arrow,
    /// Double dot (..)
    DoubleDot,
    /// Star (*)
    Star,
    /// Pipe (|)
    Pipe,
    /// Double caret (^^)
    DoubleCaret,
    /// Hat / caret (^) — property path inverse
    Hat,

    // Brackets
    /// Left bracket ([)
    LBracket,
    /// Right bracket (])
    RBracket,

    // Arithmetic operators
    /// Plus (+)
    Plus,
    /// Minus (-)
    Minus,
    /// Slash (/)
    Slash,
    /// Left parenthesis (()
    LParen,
    /// Right parenthesis ())
    RParen,

    // Comparison operators
    /// Equality (=)
    Equal,
    /// Inequality (!=)
    NotEqual,
    /// Less than (<)
    LessThan,
    /// Less than or equal (<=)
    LessEqual,
    /// Greater than (>)
    GreaterThan,
    /// Greater than or equal (>=)
    GreaterEqual,

    // Indentation tokens
    /// Indentation increase
    Indent,
    /// Indentation decrease
    Dedent,
    /// Newline
    Newline,

    // End of file
    /// End of file
    Eof,
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Package => write!(f, "package"),
            Token::OneOf => write!(f, "one of"),
            Token::Prefix => write!(f, "prefix"),
            Token::IriName => write!(f, "@iri_name"),
            Token::As => write!(f, "as"),
            Token::Concept => write!(f, "concept"),
            Token::Property => write!(f, "property"),
            Token::Rule => write!(f, "rule"),
            Token::Match => write!(f, "match"),
            Token::Then => write!(f, "then"),
            Token::Sub => write!(f, "sub"),
            Token::Has => write!(f, "has"),
            Token::Key => write!(f, "key"),
            Token::Is => write!(f, "a"),
            Token::Fact => write!(f, "fact"),
        Token::Query => write!(f, "query"),
            Token::Return => write!(f, "return"),
            Token::Either => write!(f, "either"),
            Token::Or => write!(f, "or"),
            Token::GroupBy => write!(f, "group by"),
            Token::IsInverse => write!(f, "is"),
            Token::InverseOf => write!(f, "inverse of"),
            Token::EquivalentTo => write!(f, "equivalent to"),
            Token::Transitive => write!(f, "transitive"),
            Token::Symmetric => write!(f, "symmetric"),
            Token::Reflexive => write!(f, "reflexive"),
            Token::Unit => write!(f, "unitdef"),
            Token::Family => write!(f, "family"),
            Token::Nominal => write!(f, "nominal"),
            Token::Scale => write!(f, "scale"),
            Token::All => write!(f, "all"),
            Token::None => write!(f, "none"),
            Token::AtLeast => write!(f, "at least"),
            Token::AtMost => write!(f, "at most"),
            Token::Exactly => write!(f, "exactly"),
            Token::Between => write!(f, "between"),
            Token::Of => write!(f, "of"),
            Token::One => write!(f, "one"),
            Token::Any => write!(f, "any"),
            Token::Some => write!(f, "some"),
            Token::Optional => write!(f, "optional"),
            Token::TString => write!(f, "string"),
            Token::TInt => write!(f, "int"),
            Token::TFloat => write!(f, "float"),
            Token::TBoolean => write!(f, "boolean"),
            Token::TDate => write!(f, "date"),
            Token::TDateTime => write!(f, "date_time"),
            Token::TTime => write!(f, "time"),
            Token::TDuration => write!(f, "duration"),
            Token::Int(n) => write!(f, "{}", n),
            Token::Float(n) => write!(f, "{}", n),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Boolean(b) => write!(f, "{}", b),
            Token::Iri(s) => write!(f, "<{}>", s),
            Token::TemporalLit((kind, content)) => write!(f, "{}({})", kind.keyword(), content),
            Token::QuantityLit(content) => write!(f, "quantity({})", content),
            Token::LocaleDirective(a) => write!(f, "@locale {}", a),
            Token::TimezoneDirective(a) => write!(f, "@timezone {}", a),
            Token::PrefixedName((prefix, local)) => write!(f, "{}:{}", prefix, local),
            Token::Name(s) => write!(f, "{}", s),
            Token::Variable(s) => write!(f, "{}", s),
            Token::Colon => write!(f, ":"),
            Token::Comma => write!(f, ","),
            Token::Dot => write!(f, "."),
            Token::Arrow => write!(f, "->"),
            Token::DoubleDot => write!(f, ".."),
            Token::Star => write!(f, "*"),
            Token::Pipe => write!(f, "|"),
            Token::LBracket => write!(f, "["),
            Token::RBracket => write!(f, "]"),
            Token::Plus => write!(f, "+"),
            Token::Minus => write!(f, "-"),
            Token::Slash => write!(f, "/"),
            Token::LParen => write!(f, "("),
            Token::RParen => write!(f, ")"),
            Token::Equal => write!(f, "="),
            Token::NotEqual => write!(f, "!="),
            Token::LessThan => write!(f, "<"),
            Token::LessEqual => write!(f, "<="),
            Token::GreaterThan => write!(f, ">"),
            Token::GreaterEqual => write!(f, ">="),
            Token::Indent => write!(f, "INDENT"),
            Token::Dedent => write!(f, "DEDENT"),
            Token::Newline => write!(f, "NEWLINE"),
            Token::Eof => write!(f, "EOF"),
            Token::DoubleCaret => write!(f, "^^"),
            Token::Hat => write!(f, "^"),
        }
    }
}

/// Raw token from logos lexer
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\f]+")]
// /!\ On retrire le skip des comments #[logos(skip(r"#[^\n]*", allow_greedy = true))]
pub enum RawToken {
    // Keywords
    /// 'package' keyword
    #[token("package")]
    Package,
    /// 'one of' keyword
    #[token("one of")]
    OneOf,
    /// 'prefix' keyword
    #[token("prefix")]
    Prefix,
    /// 'as' keyword
    #[token("as")]
    As,
    /// '@iri_name' annotation
    #[token("@iri_name")]
    IriName,
    /// 'concept' keyword
    #[token("concept")]
    Concept,
    /// 'property' keyword
    #[token("property")]
    Property,
    /// 'rule' keyword
    #[token("rule")]
    Rule,
    /// 'match' keyword
    #[token("match")]
    Match,
    /// 'then' keyword
    #[token("then")]
    Then,
    /// 'sub' keyword
    #[token("sub")]
    Sub,
    /// 'has' keyword
    #[token("has")]
    Has,
    /// 'key' keyword (marks a property as part of the primary key)
    #[token("key")]
    Key,
    /// 'a' keyword (type assertion)
    #[token("a")]
    Is,
    /// 'fact' keyword (instance declaration)
    #[token("fact")]
    Fact,
    /// 'query' keyword (query definition)
    #[token("query")]
    Query,
    /// 'return' keyword (query projection block)
    #[token("return")]
    Return,
    /// 'either' keyword (query disjunction)
    #[token("either")]
    Either,
    /// 'or' keyword (query disjunction branch)
    #[token("or")]
    Or,
    /// 'group by' compound keyword (query aggregation)
    #[token("group by")]
    GroupBy,
    /// 'is' keyword (inverse property assertion in fact blocks)
    #[token("is")]
    IsInverse,
    /// 'inverse of' keyword pair (property axiom)
    #[token("inverse of")]
    InverseOf,
    /// 'equivalent to' keyword pair (property axiom)
    #[token("equivalent to")]
    EquivalentTo,
    /// 'transitive' keyword (property characteristic)
    #[token("transitive")]
    Transitive,
    /// 'symmetric' keyword (property characteristic)
    #[token("symmetric")]
    Symmetric,
    /// 'reflexive' keyword (property characteristic)
    #[token("reflexive")]
    Reflexive,
    /// 'unit' keyword (unit declaration)
    #[token("unitdef")]
    Unit,
    /// 'family' keyword (unit family declaration)
    #[token("family")]
    Family,
    /// 'nominal' keyword (nominal/incommensurable unit declaration)
    #[token("nominal")]
    Nominal,
    /// 'scale' keyword (unit scale factor)
    #[token("scale")]
    Scale,

    // Quantifier keywords
    /// 'all' quantifier
    #[token("all")]
    All,
    /// 'none' quantifier
    #[token("none")]
    None,
    /// 'at least' quantifier / cardinality (underscore form kept as an alias)
    #[token("at least")]
    #[token("at_least")]
    AtLeast,
    /// 'at most' quantifier / cardinality (underscore form kept as an alias)
    #[token("at most")]
    #[token("at_most")]
    AtMost,
    /// 'exactly' quantifier
    #[token("exactly")]
    Exactly,
    /// 'of' keyword (used in 'one of:' construct)
    #[token("of")]
    Of,

    // Cardinality keywords
    /// 'one' cardinality
    #[token("one")]
    One,
    /// 'any' cardinality
    #[token("any")]
    Any,
    /// 'some' cardinality
    #[token("some")]
    Some,
    /// 'optional' cardinality
    #[token("optional")]
    Optional,

    // Primitive type keywords
    /// 'string' type keyword
    #[token("string")]
    TString,
    /// 'int' type keyword
    #[token("int")]
    TInt,
    /// 'float' type keyword
    #[token("float")]
    TFloat,
    /// 'boolean' type keyword
    #[token("boolean")]
    TBoolean,
    /// 'date' type keyword
    #[token("date")]
    TDate,
    /// 'date_time' type keyword
    #[token("date_time")]
    TDateTime,
    /// 'time' type keyword
    #[token("time")]
    TTime,
    /// 'duration' type keyword
    #[token("duration")]
    TDuration,

    // Boolean literals
    /// 'true' literal
    #[token("true")]
    True,
    /// 'false' literal
    #[token("false")]
    False,

    // Float (must come before Int to match properly)
    /// Floating-point literal
    #[regex(r"[0-9][0-9_]*\.[0-9][0-9_]*([eE][+-]?[0-9]+)?", parse_float)]
    Float(f64),

    // Integer
    /// Integer literal
    #[regex(r"[0-9][0-9_]*", parse_int)]
    Int(i64),

    /// Temporal smart literal: `date(...)`, `time(...)`, `date_time(...)`,
    /// `duration(...)`. The whole `keyword(content)` is slurped as one raw
    /// token; `content` is handed verbatim to `dolfin-datetime`. Longest-match
    /// beats the bare `date`/`time`/`duration` type keywords, which only match
    /// when no `(` follows.
    #[regex(r"(date_time|date|time|duration)\([^)\n]*\)", parse_temporal_lit)]
    TemporalLit((TemporalKind, String)),

    /// Physical-quantity smart literal: `quantity(42 km/h)`,
    /// `quantity(10 N/m^2)`, `quantity(42 km/h as m/s)`. The whole
    /// `quantity(content)` is slurped as one raw token; `content` is handed
    /// verbatim to `dolfin-units`. The inner alternation allows one level of
    /// nested parens so exponent forms like `m.s^(-2)` are captured whole.
    /// A bare `quantity` with no `(` following lexes as a plain `Name`.
    #[regex(r"quantity\((?:[^()\n]|\([^()\n]*\))*\)", parse_quantity_lit)]
    QuantityLit(String),

    /// `@locale d/m/y` file-level directive. The argument (everything up to a
    /// trailing comment or end of line) is slurped raw and parsed later.
    #[regex(r"@locale[ \t]+[^\n]*", parse_locale_directive, allow_greedy = true)]
    LocaleDirective(String),

    /// `@timezone Europe/Brussels` file-level directive.
    #[regex(r"@timezone[ \t]+[^\n]*", parse_timezone_directive, allow_greedy = true)]
    TimezoneDirective(String),

    /// String literal
    #[regex(r#""([^"\\]|\\.)*""#, parse_string)]
    String(String),

    // IRI
    /// IRI literal
    #[regex(r"<[^>]+>", parse_iri)]
    Iri(String),

    // Variable (starts with ?)
    /// Variable reference
    #[regex(r"\?[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Variable(String),

    /// Prefixed name (alias:LocalName, no whitespace around colon)
    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*:[a-zA-Z][a-zA-Z0-9_]*", parse_prefixed_name, priority = 2)]
    PrefixedName((String, String)),

    /// Identifier name
    #[regex(r"[a-zA-Z][a-zA-Z0-9_]*", |lex| lex.slice().to_string(), priority = 1)]
    Name(String),

    // Punctuation (multi-char first)
    /// Arrow operator (->)
    #[token("->")]
    Arrow,
    /// Double dot operator (..)
    #[token("..")]
    DoubleDot,
    /// Not equal operator (!=)
    #[token("!=")]
    NotEqual,
    /// Less than or equal operator (<=)
    #[token("<=")]
    LessEqual,
    /// Greater than or equal operator (>=)
    #[token(">=")]
    GreaterEqual,
    /// Colon (:)
    #[token(":")]
    Colon,
    /// Comma (,)
    #[token(",")]
    Comma,
    /// Dot (.)
    #[token(".")]
    Dot,
    /// Star (*)
    #[token("*")]
    Star,
    /// Pipe (|)
    #[token("|")]
    Pipe,
    /// Left bracket ([)
    #[token("[")]
    LBracket,
    /// Right bracket (])
    #[token("]")]
    RBracket,
    /// Equality operator (=)
    #[token("=")]
    Equal,
    /// Less than operator (<)
    #[token("<")]
    LessThan,
    /// Greater than operator (>)
    #[token(">")]
    GreaterThan,
    /// Double caret (^^)
    #[token("^^")]
    DoubleCaret,
    /// Hat / caret (^) — property path inverse prefix
    #[token("^")]
    Hat,
    /// Plus operator (+)
    #[token("+")]
    Plus,
    /// Minus operator (-)
    #[token("-")]
    Minus,
    /// Slash operator (/)
    #[token("/")]
    Slash,
    /// Left parenthesis (()
    #[token("(")]
    LParen,
    /// Right parenthesis ())
    #[token(")")]
    RParen,

    // Newline (captures the newline and any following indentation)
    /// Newline with optional following indentation
    #[regex(r"(\r?\n[ \t]*)+", |lex| lex.slice().to_string())]
    Newline(String),

    #[regex(r"[ \t]*#[^\n]*", allow_greedy = true)]
    Comment,
}

fn parse_prefixed_name(lex: &logos::Lexer<RawToken>) -> Option<(String, String)> {
    let slice = lex.slice();
    let mut parts = slice.splitn(2, ':');
    let prefix = parts.next()?.to_string();
    let local = parts.next()?.to_string();
    Some((prefix, local))
}

fn parse_int(lex: &logos::Lexer<RawToken>) -> Option<i64> {
    let slice = lex.slice();
    let cleaned: String = slice.chars().filter(|c| *c != '_').collect();
    cleaned.parse().ok()
}

fn parse_float(lex: &logos::Lexer<RawToken>) -> Option<f64> {
    let slice = lex.slice();
    let cleaned: String = slice.chars().filter(|c| *c != '_').collect();
    cleaned.parse().ok()
}

fn parse_string(lex: &logos::Lexer<RawToken>) -> Option<String> {
    let slice = lex.slice();
    // Remove quotes and handle escape sequences
    let inner = &slice[1..slice.len() - 1];
    let mut result = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('t') => result.push('\t'),
                Some('r') => result.push('\r'),
                Some('\\') => result.push('\\'),
                Some('"') => result.push('"'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                }
                None => result.push('\\'),
            }
        } else {
            result.push(c);
        }
    }
    Some(result)
}

fn parse_iri(lex: &logos::Lexer<RawToken>) -> Option<String> {
    let slice = lex.slice();
    Some(slice[1..slice.len() - 1].to_string())
}

/// Extract the argument of a `@directive <arg>` line, stripping the keyword and
/// any trailing `# comment`.
fn directive_arg(slice: &str, keyword: &str) -> Option<String> {
    let arg = slice.strip_prefix(keyword)?.trim();
    let arg = arg.split('#').next().unwrap_or(arg).trim();
    if arg.is_empty() {
        None
    } else {
        Some(arg.to_string())
    }
}

fn parse_locale_directive(lex: &logos::Lexer<RawToken>) -> Option<String> {
    directive_arg(lex.slice(), "@locale")
}

fn parse_timezone_directive(lex: &logos::Lexer<RawToken>) -> Option<String> {
    directive_arg(lex.slice(), "@timezone")
}

/// Split a slurped `keyword(content)` temporal literal into its declared kind
/// and the raw content between the parentheses.
fn parse_temporal_lit(lex: &logos::Lexer<RawToken>) -> Option<(TemporalKind, String)> {
    let slice = lex.slice();
    let open = slice.find('(')?;
    let kind = match &slice[..open] {
        "date" => TemporalKind::Date,
        "time" => TemporalKind::Time,
        "date_time" => TemporalKind::DateTime,
        "duration" => TemporalKind::Duration,
        _ => return None,
    };
    let content = slice[open + 1..slice.len() - 1].trim().to_string();
    Some((kind, content))
}

/// Split a slurped `quantity(content)` literal into its raw content, trimmed of
/// the `quantity(` prefix and the trailing `)`. The content is handed verbatim
/// to `dolfin-units`.
fn parse_quantity_lit(lex: &logos::Lexer<RawToken>) -> Option<String> {
    let slice = lex.slice();
    let open = slice.find('(')?;
    Some(slice[open + 1..slice.len() - 1].trim().to_string())
}

/// Spanned token
pub type Spanned<T> = (Location, T, Location);

/// The Dolfin lexer with indentation handling
pub struct Lexer<'input> {
    /// Source code being lexed
    source: &'input str,
    /// The underlying logos lexer
    logos_lexer: logos::Lexer<'input, RawToken>,
    /// Stack of indentation levels
    indent_stack: Vec<usize>,
    /// Tokens waiting to be emitted
    pending_tokens: VecDeque<Spanned<Token>>,
    /// Errors waiting to be emitted
    pending_errors: VecDeque<LexerError>,
    /// Byte offsets of every `\n` seen so far, in ascending order.
    /// Used for O(log n) line/column lookup via binary search.
    newline_offsets: Vec<usize>,
    /// Are we at the start of a line?
    at_line_start: bool,
    /// Have we finished lexing?
    finished: bool,
    /// side-channel for comments
    comment_sink: CommentSink,
    previous_token_was_comment: bool,
}

impl<'input> Lexer<'input> {
    /// Create a new lexer for the given source code
    pub fn new(source: &'input str) -> Self {
        Self::with_comment_sink(source, CommentSink::new())
    }

    pub fn with_comment_sink(source: &'input str, sink: CommentSink) -> Self {
        Self {
            source,
            logos_lexer: RawToken::lexer(source),
            indent_stack: vec![0],
            pending_tokens: VecDeque::new(),
            pending_errors: VecDeque::new(),
            newline_offsets: Vec::new(),
            at_line_start: true,
            finished: false,
            comment_sink: sink,
            previous_token_was_comment: false,
        }
    }

    /// Consume the lexer and return the collected comments.
    /// Called after parsing is complete.
    pub fn into_comments(self) -> CommentSink {
        self.comment_sink
    }

    /// Borrow the collected comments (useful during parsing).
    pub fn comment_sink(&self) -> &CommentSink {
        &self.comment_sink
    }

    /// Convert a byte offset into a 1-based `Location` using the recorded
    /// newline offsets.  O(log n) via binary search.
    fn location_from_offset(&self, offset: usize) -> Location {
        // Number of newlines strictly before `offset` = the 0-based line index.
        let line_idx = self.newline_offsets.partition_point(|&nl| nl < offset);
        let line_start = if line_idx == 0 {
            0
        } else {
            self.newline_offsets[line_idx - 1] + 1 // byte after the '\n'
        };
        Location {
            line: line_idx + 1,              // 1-based
            column: offset - line_start + 1, // 1-based
            offset,
        }
    }

    /// Record the byte offset of every `\n` in `text`, where `text` starts at
    /// `span_start` in the source.  Must be called in source order so that
    /// `newline_offsets` stays sorted.
    fn record_newlines(&mut self, span_start: usize, text: &str) {
        let mut pos = span_start;
        for c in text.chars() {
            if c == '\n' {
                self.newline_offsets.push(pos);
            }
            pos += c.len_utf8();
        }
    }

    fn calculate_indent(s: &str) -> usize {
        let mut indent = 0;
        for c in s.chars() {
            match c {
                ' ' => indent += 1,
                '\t' => indent += 2,
                '\n' | '\r' => indent = 0,
                _ => break,
            }
        }
        indent
    }

    fn emit_indentation_tokens(&mut self, new_indent: usize, loc: Location) {
        let current_indent = *self.indent_stack.last().unwrap_or(&0);

        if new_indent > current_indent {
            self.indent_stack.push(new_indent);
            self.pending_tokens.push_back((loc, Token::Indent, loc));
        } else if new_indent < current_indent {
            // Emit DEDENT tokens for each level we're leaving
            while let Some(&top) = self.indent_stack.last() {
                if top <= new_indent {
                    break;
                }
                self.indent_stack.pop();
                self.pending_tokens.push_back((loc, Token::Dedent, loc));
            }
        }
    }

    fn convert_raw_token(&self, raw: RawToken) -> Token {
        match raw {
            RawToken::Package => Token::Package,
            RawToken::OneOf => Token::OneOf,
            RawToken::Prefix => Token::Prefix,
            RawToken::IriName => Token::IriName,
            RawToken::As => Token::As,
            RawToken::Concept => Token::Concept,
            RawToken::Property => Token::Property,
            RawToken::Rule => Token::Rule,
            RawToken::Match => Token::Match,
            RawToken::Then => Token::Then,
            RawToken::Sub => Token::Sub,
            RawToken::Has => Token::Has,
            RawToken::Key => Token::Key,
            RawToken::Is => Token::Is,
            RawToken::Fact => Token::Fact,
            RawToken::Query => Token::Query,
            RawToken::Return => Token::Return,
            RawToken::Either => Token::Either,
            RawToken::Or => Token::Or,
            RawToken::GroupBy => Token::GroupBy,
            RawToken::IsInverse => Token::IsInverse,
            RawToken::InverseOf => Token::InverseOf,
            RawToken::EquivalentTo => Token::EquivalentTo,
            RawToken::Unit => Token::Unit,
                RawToken::Family => Token::Family,
                RawToken::Nominal => Token::Nominal,
                RawToken::Scale => Token::Scale,
                RawToken::Transitive => Token::Transitive,
            RawToken::Symmetric => Token::Symmetric,
            RawToken::Reflexive => Token::Reflexive,
            RawToken::All => Token::All,
            RawToken::None => Token::None,
            RawToken::AtLeast => Token::AtLeast,
            RawToken::AtMost => Token::AtMost,
            RawToken::Exactly => Token::Exactly,
            RawToken::Of => Token::Of,
            RawToken::One => Token::One,
            RawToken::Any => Token::Any,
            RawToken::Some => Token::Some,
            RawToken::Optional => Token::Optional,
            RawToken::TString => Token::TString,
            RawToken::TInt => Token::TInt,
            RawToken::TFloat => Token::TFloat,
            RawToken::TBoolean => Token::TBoolean,
            RawToken::TDate => Token::TDate,
            RawToken::TDateTime => Token::TDateTime,
            RawToken::TTime => Token::TTime,
            RawToken::TDuration => Token::TDuration,
            RawToken::True => Token::Boolean(true),
            RawToken::False => Token::Boolean(false),
            RawToken::Int(n) => Token::Int(n),
            RawToken::Float(f) => Token::Float(f),
            RawToken::String(s) => Token::String(s),
            RawToken::Iri(s) => Token::Iri(s),
            RawToken::TemporalLit(t) => Token::TemporalLit(t),
            RawToken::QuantityLit(c) => Token::QuantityLit(c),
            RawToken::LocaleDirective(a) => Token::LocaleDirective(a),
            RawToken::TimezoneDirective(a) => Token::TimezoneDirective(a),
            RawToken::Variable(v) => Token::Variable(v),
            RawToken::PrefixedName(p) => Token::PrefixedName(p),
            RawToken::Name(n) => Token::Name(n),
            RawToken::Arrow => Token::Arrow,
            RawToken::DoubleDot => Token::DoubleDot,
            RawToken::NotEqual => Token::NotEqual,
            RawToken::LessEqual => Token::LessEqual,
            RawToken::GreaterEqual => Token::GreaterEqual,
            RawToken::Colon => Token::Colon,
            RawToken::Comma => Token::Comma,
            RawToken::Dot => Token::Dot,
            RawToken::Star => Token::Star,
            RawToken::Pipe => Token::Pipe,
            RawToken::LBracket => Token::LBracket,
            RawToken::RBracket => Token::RBracket,
            RawToken::Equal => Token::Equal,
            RawToken::LessThan => Token::LessThan,
            RawToken::GreaterThan => Token::GreaterThan,
            RawToken::Newline(_) => Token::Newline,
            RawToken::DoubleCaret => Token::DoubleCaret,
            RawToken::Hat => Token::Hat,
            RawToken::Plus => Token::Plus,
            RawToken::Minus => Token::Minus,
            RawToken::Slash => Token::Slash,
            RawToken::LParen => Token::LParen,
            RawToken::RParen => Token::RParen,
            RawToken::Comment => unreachable!("comments should be filtered before conversion"),
        }
    }

    fn compact_pending_tokens(&mut self) {
        if self.pending_tokens.len() <= 1 {
            return;
        }
        let mut stack = VecDeque::new();
        let mut new_pending_tokens = VecDeque::new();
        if let Some(front) = self.pending_tokens.pop_front() {
            new_pending_tokens.push_back(front);
        } else {
            return;
        }
        while !self.pending_tokens.is_empty() {
            match new_pending_tokens.pop_back() {
                Some((start, Token::Newline, end)) => match self.pending_tokens.pop_front() {
                    Some((_, Token::Newline, nend)) => {
                        new_pending_tokens.push_back((start, Token::Newline, nend));
                    }
                    Some((nstart, Token::Dedent, nend)) => {
                        if stack.is_empty() {
                            new_pending_tokens.push_back((start, Token::Newline, end));
                            new_pending_tokens.push_back((nstart, Token::Dedent, nend));
                        } else {
                            if let Some(last) = stack.pop_back() {
                                let mut new = start;
                                while new_pending_tokens.len() > last {
                                    if let Some((update_new, _, _)) = new_pending_tokens.pop_back()
                                    {
                                        new = update_new;
                                    }
                                }
                                self.pending_tokens.push_front((new, Token::Newline, nend));
                            }
                        }
                    }
                    Some(value) => {
                        new_pending_tokens.push_back((start, Token::Newline, end));
                        new_pending_tokens.push_back(value);
                    }
                    _ => todo!("We should not be here"),
                },
                Some((start, Token::Indent, end)) => match self.pending_tokens.pop_front() {
                    Some((_, Token::Dedent, nend)) => {
                        self.pending_tokens
                            .push_front((start, Token::Newline, nend));
                    }
                    Some((_, Token::Newline, _)) => {
                        new_pending_tokens.push_back((start, Token::Indent, end));
                    }
                    Some(t) => {
                        let here = new_pending_tokens.len();
                        if let Some(there) = stack.pop_back()
                            && here != there
                        {
                            stack.push_back(there);
                            stack.push_back(here);
                        } else {
                            stack.push_back(here);
                        }
                        new_pending_tokens.push_back((start, Token::Indent, end));
                        new_pending_tokens.push_back(t);
                    }
                    _ => {
                        todo!("we shouldn't be here");
                    }
                },
                Some(t) => {
                    new_pending_tokens.push_back(t);
                    if let Some(t2) = self.pending_tokens.pop_front() {
                        new_pending_tokens.push_back(t2);
                    } else {
                        todo!("We shouldn't be here")
                    }
                }
                _ => {
                    todo!("Wh shouldn't be here")
                }
            }
        }
        self.pending_tokens = new_pending_tokens.clone();
    }

    fn should_emit_tokens(&mut self) -> bool {
        let peek_pending_tokens: Vec<Token> = self
            .pending_tokens
            .iter()
            .map(|(_, t, _)| t.clone())
            .collect();
        self.compact_pending_tokens();
        peek_pending_tokens
            .iter()
            .cloned()
            .filter(|t| *t != Token::Newline && *t != Token::Indent && *t != Token::Dedent)
            .count()
            != 0
    }

    fn next_token(&mut self) -> Option<Result<Spanned<Token>, LexerError>> {
        // Return pending errors first (they take priority)
        if let Some(err) = self.pending_errors.pop_front() {
            return Some(Err(err));
        }

        // Return pending tokens first
        if self.should_emit_tokens() {
            if let Some(tok) = self.pending_tokens.pop_front() {
                return Some(Ok(tok));
            }
        }

        if self.finished {
            return None;
        }

        loop {
            match self.logos_lexer.next() {
                Some(Ok(raw_token)) => {
                    let span = self.logos_lexer.span();
                    // start_loc is computed before we record any newlines from
                    // this token, so it correctly reflects state up to this point.
                    let start_loc = self.location_from_offset(span.start);

                    match raw_token {
                        RawToken::Comment => {
                            // Comments contain no newlines (pattern: #[^\n]*),
                            // so no newline recording needed.
                            let end_loc = self.location_from_offset(span.end);
                            let raw_text = self.source[span.clone()].to_string();
                            let text = raw_text
                                .trim_start()
                                .trim_start_matches('#')
                                .trim_start()
                                .to_string();
                            let raw_text = raw_text + "\n";
                            let comment = Comment {
                                text,
                                raw: raw_text,
                                span: Span::new(start_loc, end_loc),
                                line: start_loc.line,
                                column: start_loc.column,
                            };
                            if self.previous_token_was_comment {
                                self.comment_sink.push_and_merge(comment);
                            } else {
                                self.comment_sink.push(comment);
                            }
                            self.previous_token_was_comment = true;
                            // Loop again to find the next real token
                            continue;
                        }
                        RawToken::Newline(ref nl_text) => {
                            // self.previous_token_was_comment = false;
                            // Record newline positions BEFORE computing end_loc
                            // so that end_loc (which is on the next line) resolves
                            // correctly via binary search.
                            self.record_newlines(span.start, nl_text);
                            let end_loc = self.location_from_offset(span.end);

                            let new_indent = Self::calculate_indent(nl_text);

                            // Emit
                            let previous = self.pending_tokens.pop_back();
                            if let Some((p_start, Token::Newline, _)) = previous {
                                self.pending_tokens
                                    .push_back((p_start, Token::Newline, end_loc));
                            } else {
                                if let Some(previous) = previous {
                                    self.pending_tokens.push_back(previous);
                                }
                                self.pending_tokens
                                    .push_back((start_loc, Token::Newline, end_loc));
                            }

                            // Comment-only lines must not affect indentation
                            // tracking: their leading whitespace is whatever the
                            // author happened to type, not a real block level.
                            // Peek past this NEWLINE and skip the indent-stack
                            // update if a Comment follows — the NEWLINE after the
                            // comment (preceding the next real line) will supply
                            // the indentation that actually matters.
                            let next_is_comment = matches!(
                                self.logos_lexer.clone().next(),
                                Some(Ok(RawToken::Comment))
                            );

                            // Emit indentation changes
                            if !next_is_comment {
                                self.emit_indentation_tokens(new_indent, end_loc);
                            }

                            self.at_line_start = true;
                            if !self.should_emit_tokens() {
                                continue;
                            } else {
                                return self.pending_tokens.pop_front().map(Ok);
                            }
                        }
                        _ => {
                            self.previous_token_was_comment = false;
                            let end_loc = self.location_from_offset(span.end);
                            self.at_line_start = false;
                            let token = self.convert_raw_token(raw_token);
                            self.pending_tokens.push_back((start_loc, token, end_loc));
                            return self.pending_tokens.pop_front().map(Ok);
                        }
                    }
                }
                Some(Err(())) => {
                    // Logos couldn't match any token
                    let span = self.logos_lexer.span();
                    let bad_char = &self.source[span.clone()];
                    let loc = self.location_from_offset(span.start);
                    return Some(Err(LexerError::new(
                        format!("Unrecognized character: '{}'", bad_char.escape_default()),
                        loc,
                        ErrorCode::InvalidCharacter,
                    )));
                }
                None => {
                    // End of input - emit final DEDENTs
                    self.finished = true;
                    let loc = self.location_from_offset(self.source.len());

                    // Emit NEWLINE if not at line start
                    if !self.at_line_start {
                        self.pending_tokens.push_back((loc, Token::Newline, loc));
                    }

                    // Emit remaining DEDENTs
                    while self.indent_stack.len() > 1 {
                        self.indent_stack.pop();
                        self.pending_tokens.push_back((loc, Token::Dedent, loc));
                    }

                    // Emit EOF
                    self.pending_tokens.push_back((loc, Token::Eof, loc));
                    return self.pending_tokens.pop_front().map(Ok);
                }
            }
        }
    }
}

impl<'input> Iterator for Lexer<'input> {
    type Item = Result<Spanned<Token>, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        let nt = self.next_token();
        debug!("{:?}", nt);
        nt
    }
}
