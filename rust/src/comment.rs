use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{Declaration, OntologyFile, error::Span};

/// A single comment from the source.
#[cfg_attr(feature = "python", pyo3::pyclass(from_py_object, get_all))]
#[derive(Debug, Clone, PartialEq)]
pub struct Comment {
    /// The text content, without the leading `#` and without trailing newline.
    pub text: String,
    /// Original text including the `#`.
    pub raw: String,
    /// Position in source.
    pub span: Span,
    /// Line number (1-based) where this comment appears.
    pub line: usize,
    /// Column where the `#` starts (1-based).
    pub column: usize,
}

/// Where a comment sits relative to a code node.
#[derive(Debug, Clone, PartialEq)]
pub enum CommentPlacement {
    /// Comment on a line before the node, at the same or deeper indentation.
    ///
    /// ```dolfin
    /// # This describes Person
    /// concept Person:
    /// ```
    Leading,

    /// Comment on the same line, after the code.
    ///
    /// ```dolfin
    /// has name: string  # required field
    /// ```
    Trailing,

    /// Comment at the beginning of a block, that
    /// can't be seen at Leading nor Trailing.
    ///
    /// ```dolfin
    /// has name: string
    ///   # anything here
    /// ```
    Inside,

    /// Comment inside an empty block, or between nodes where
    /// it doesn't clearly belong to either.
    ///
    /// ```dolfin
    /// concept Empty:
    ///   # TODO: add members
    /// ```
    Dangling,
}

/// A comment attached to a specific AST node.
#[derive(Debug, Clone)]
pub struct AttachedComment {
    pub comment: Comment,
    pub placement: CommentPlacement,
}

#[derive(Debug, Clone, Default)]

/// Shared comment sink that survives the lexer being consumed.
pub struct CommentSink {
    inner: Rc<RefCell<Vec<Comment>>>,
}

impl CommentSink {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn push(&self, comment: Comment) {
      self.inner.borrow_mut().push(comment);
    }

    pub fn push_and_merge(&self, comment: Comment) {
      let mut inner = self.inner.borrow_mut();
      if let Some(previous) = inner.pop() {
        if previous.span.end.line + 1 == comment.line && previous.column == comment.column {
          let new_comment = Comment {
            text: format!("{}\n{}", previous.text, comment.text),
            raw: format!("{}{}", previous.raw, comment.raw),
            span: previous.span.merge(&comment.span),
            line: previous.line,
            column: previous.column,
          };
          inner.push(new_comment);
        } else {
          inner.push(previous);
          inner.push(comment);
        }
      } else {
        inner.push(comment);
      }
    }

    /// Extract all collected comments.
    pub fn take(&self) -> Vec<Comment> {
        std::mem::take(&mut *self.inner.borrow_mut())
    }

    pub fn comments(&self) -> Vec<Comment> {
        self.inner.borrow().clone()
    }
}

/// Map from AST node spans to their attached comments.
#[derive(Debug, Default, Clone)]
pub struct CommentMap {
    /// Leading comments: appear on lines before the node.
    pub leading: HashMap<Span, Vec<Comment>>,
    /// Trailing comments: appear on the same line after the node.
    pub trailing: HashMap<Span, Vec<Comment>>,
    /// Inside comments: appear at the beginning of a indented block.
    /// being neither leading nor trailing.
    pub inside: HashMap<Span, Vec<Comment>>,
    /// Dangling comments: inside empty blocks or unattachable.
    pub dangling: Vec<Comment>,
}

impl CommentMap {
    /// Build the comment map from an AST and collected comments.
    pub fn build(ontology: &OntologyFile, mut comments: Vec<Comment>) -> Self {
        if comments.is_empty() {
            return Self::default();
        }

        // Sort comments by position
        comments.sort_by_key(|c| c.span);

        // collect all node sspans from the AST, sorted
        let mut node_spans = Vec::new();
        collect_spans(&ontology, &mut node_spans);
        node_spans.sort();

        let mut map = CommentMap::default();
        let mut unattached: Vec<Comment> = Vec::new();
        for comment in comments {
            match classify_comment(&comment, &node_spans) {
                Some((placement, node_span)) => {
                    let bucket = match placement {
                        CommentPlacement::Leading => map.leading.entry(node_span).or_default(),
                        CommentPlacement::Trailing => map.trailing.entry(node_span).or_default(),
                        CommentPlacement::Inside => map.inside.entry(node_span).or_default(),
                        CommentPlacement::Dangling => {
                            map.dangling.push(comment);
                            continue;
                        }
                    };
                    bucket.push(comment);
                }
                None => {
                    unattached.push(comment);
                }
            }
        }

        map.dangling.extend(unattached);
        map
    }

    /// Get leading comments for a node.
    pub fn leading_comments(&self, span: &Span) -> &[Comment] {
        self.leading.get(span).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn trailing_comments(&self, span: &Span) -> &[Comment] {
        self.trailing.get(span).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn dangling_comments(&self, span: &Span) -> Vec<Comment> {
        self.dangling
            .iter()
            .filter(|v| {
                span.start.offset <= v.span.start.offset && v.span.end.offset <= span.end.offset
            })
            .cloned()
            .collect()
    }

    pub fn inside_comments(&self, span: &Span) -> &[Comment] {
        self.inside.get(span).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

/// Determine where a comment belongs relative to the known node spans.
fn classify_comment(comment: &Comment, node_spans: &[Span]) -> Option<(CommentPlacement, Span)> {
    let comment_line = comment.line;
    let comment_offset = comment.span.start.offset;
    let comment_end_offset: usize = comment.span.end.offset;
    let comment_lines_count = comment.text.split('\n').count();
    for span in node_spans.iter().rev() {
        // 1. Trailing: is there a node that ENDS on the same line,
        //              before the comment ?
        //              e.g., `has name: string # required`
        if span.end.line == comment_line && span.end.offset <= comment_offset {
            return Some((CommentPlacement::Trailing, *span));
        }
        //    Trailing: is there a comments on the line above that
        //              starts on the same column and no spans exists
        //              between it and us ?
        //              e.g., `has name: string # required`
        //                    `                 # it means mandatory`
        if span.end.offset <= comment_offset
            && node_spans
                .iter()
                .filter(|s| span < s && s < &&comment.span)
                .count()
                == 0
            && span.end.column + 4 >= comment.span.start.column
            && span.end.column <= comment.span.start.column
        {
            return Some((CommentPlacement::Trailing, *span));
        }
    }

    // 2. Leading: find the next node that STARTS after the comment.
    //    The comment "leads into" that node.
    //    e.g., `# Describes a person`
    //          `concept Person:`
    let next_node = node_spans
        .iter()
        .find(|span| span.start.offset > comment_end_offset);
    let mut postponed = None;
    if let Some(next) = next_node {
        // Only attach as leading if the comment is "close" to the node.
        // Heuristic: comment must be within 1 blank line of the node.
        let line_gap = next.start.line.saturating_sub(comment_line);
        if line_gap == comment_lines_count {
            return Some((CommentPlacement::Leading, *next));
        }
        // Heuristic: if there is one blank line gap then we postpone the return
        // to check if the comments will not be choosen for a inside.
        if line_gap == 1 + comment_lines_count {
            postponed = Some((CommentPlacement::Leading, *next));
        }
    }

    let mut the_last_return = None;
    for span in node_spans.iter().rev() {
        //    Inside:   is the line above that starts on a column
        //              before us and no spans exists between it and us ?
        //              e.g., `has name: string`
        //                    `  # say my name, say my name, ...`
        if span.end.offset <= comment_offset
            && node_spans
                .iter()
                .filter(|s| span < s && s < &&comment.span)
                .count()
                == 0
            && span.start.column < comment.span.start.column
        {
            the_last_return = Some((CommentPlacement::Inside, *span));
        }
        //    Inside:   inside a span
        //              and no spans inside it exists between it and us ?
        //              e.g., `concept Flower:`
        //                    `  # Spring power !`
        //                    `  `
        //                    `  has color: Color`
        if span.start.offset <= comment_offset
            && comment_end_offset <= span.end.offset
            && node_spans
                .iter()
                .filter(|s| {
                    span.start.offset < s.start.offset
                        && s.end.offset <= span.end.offset
                        && s < &&comment.span
                })
                .count()
                == 0
            && span.start.column < comment.span.start.column
        {
            the_last_return = Some((CommentPlacement::Inside, *span));
        }
    }

    if let Some(_) = the_last_return {
      return the_last_return;
    }

    if let Some(_) = postponed {
        return postponed;
    }

    // 3. Is the comment inside a node's span? -> Dangling.
    for span in node_spans.iter() {
        if span.start.offset < comment_offset && comment_offset < span.end.offset {
            return Some((CommentPlacement::Dangling, *span));
        }
    }

    // 4. Unattachable (e.g., comment at the very end of file with no follwing node).
    //    Attach to previous node a trailing if possible.
    let prev_node = node_spans
        .iter()
        .rev()
        .find(|span| span.end.offset <= comment_offset);

    if let Some(prev) = prev_node {
        let line_gap = comment_line.saturating_sub(prev.end.line);
        if line_gap <= 1 {
            return Some((CommentPlacement::Trailing, *prev));
        }
    }

    None // Truly dangling - file-level comment
}

/// Walk the AST and collect all node spans.
fn collect_spans(ontology: &OntologyFile, out: &mut Vec<Span>) {
    if let Some(span) = ontology.span {
        out.push(span);
    }
    for prefix in &ontology.prefixes {
        if let Some(span) = prefix.span {
            out.push(span);
        }
    }

    for decl in &ontology.declarations {
        collect_declaration_spans(decl, out);
    }
}

fn collect_declaration_spans(decl: &Declaration, out: &mut Vec<Span>) {
    match decl {
        Declaration::Concept(concept_def) => {
            if let Some(span) = concept_def.span {
                out.push(span);
            }
            for hd in &concept_def.has_declarations {
                if let Some(span) = hd.span {
                    out.push(span);
                }
            }
            for sd in &concept_def.parents {
                match sd {
                    crate::TypeRef::Named { name: _, span } => {
                        if let Some(span) = span {
                            out.push(*span);
                        }
                    }
                    crate::TypeRef::Primitive { kind: _, span } => {
                        if let Some(span) = span {
                            out.push(*span);
                        }
                    }
                }
            }
            if let Some(one_of) = &concept_def.one_of {
              for oo in one_of {
                if let Some(span) = oo.span {
                  out.push(span);
                }
              }
            }
        }
        Declaration::Property(property_def) => {
            if let Some(span) = property_def.span {
                out.push(span);
            }
        }
        Declaration::Rule(rule_def) => {
            if let Some(span) = rule_def.span {
                out.push(span);
            }
            // Recurse into match/then blocks...
        }
    }
}
