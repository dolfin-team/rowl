//! Abstract Syntax Tree definitions for the Dolfin language.
//!
//! All AST nodes are exposed to Python via PyO3.

#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::fmt;

use crate::{error::{Span, SpannedString}, macros::impl_python};

/// A qualified (dot-separated) name in Dolfin.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct QualifiedName {
    /// The individual components of the qualified name (e.g., ["com", "example", "Person"])
    pub parts: Vec<String>,
    /// Source code span for this declaration
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl QualifiedName {
    #[new]
    #[pyo3(signature = (parts, span=None))]
    pub fn new(parts: Vec<String>, span: Option<Span>) -> Self {
        Self { parts, span }
    }

    #[staticmethod]
    pub fn from_single(name: String) -> Self {
        Self { parts: vec![name], span: None }
    }

    /// Get the prefix (all parts except the last)
    #[getter]
    pub fn prefix(&self) -> Option<String> {
        if self.parts.len() > 1 {
            Some(self.parts[..self.parts.len() - 1].join("."))
        } else {
            None
        }
    }

    /// Get the last component of the name
    #[getter]
    pub fn last(&self) -> String {
        self.parts.last().cloned().unwrap_or_default()
    }

    /// Get the full qualified name as a string
    #[getter]
    pub fn full(&self) -> String {
        self.parts.join(".")
    }

    /// Get the full qualified name as a string
    #[getter]
    pub fn full_slashed(&self) -> String {
        self.parts.join("/")
    }

    /// Combine two qualified names
    pub fn join(&self, other: &QualifiedName) -> QualifiedName {
        let mut parts = self.parts.clone();
        parts.extend(other.parts.iter().cloned());
        if let Some(ss) = &self.span && let Some(os) = &other.span {
            QualifiedName { parts, span: Some(ss.merge(os)) }
        } else {
            QualifiedName { parts, span: None }
        }
    }

    /// Remove last part and give a QualifiedName
    pub fn prefix_as_qn(&self) -> Option<QualifiedName> {
      if self.parts.len() > 1 {
        Some(QualifiedName { parts: self.parts[self.parts.len() - 1..].to_vec(), span: None })
      } else {
        None
      }
    }

    fn __repr__(&self) -> String {
        format!("QualifiedName('{}')", self.full())
    }

    fn __str__(&self) -> String {
        self.full()
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.parts == other.parts
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.parts.hash(&mut hasher);
        hasher.finish()
    }
}
}

impl QualifiedName {
  pub fn strip_prefix(&self, other: &QualifiedName) -> QualifiedName {
    let parts = if self.parts.starts_with(&other.parts[..]) {
      self.parts[other.parts.len()..].to_vec()
    } else {
      self.parts.clone()
    };
    QualifiedName { parts, span: None }
  }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full())
    }
}


/// An ontology file (new top-level structure)
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct OntologyFile {
    /// Optional IRI name override from @iri_name annotation
    pub iri_name: Option<String>,
    /// Prefix declarations
    pub prefixes: Vec<PrefixDecl>,
    /// Declarations (concepts, properties, enums, rules)
    pub declarations: Vec<Declaration>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl OntologyFile {
    /// Create a new ontology file
    #[new]
    #[pyo3(signature = (iri_name=None, prefixes=vec![], declarations=vec![], span=None))]
    pub fn new(
        iri_name: Option<String>,
        prefixes: Vec<PrefixDecl>,
        declarations: Vec<Declaration>,
        span: Option<Span>,
    ) -> Self {
        Self { iri_name, prefixes, declarations, span }
    }

    fn __repr__(&self) -> String {
        format!(
            "OntologyFile(iri_name={:?}, prefixes={}, declarations={})",
            self.iri_name,
            self.prefixes.len(),
            self.declarations.len()
        )
    }

    #[getter]
    /// Get all concepts
    pub fn concepts(&self) -> Vec<ConceptDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Concept(c) => Some(c.clone()),
                _ => None,
            })
            .collect()
    }

    #[getter]
    /// Get all properties
    pub fn properties(&self) -> Vec<PropertyDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Property(p) => Some(p.clone()),
                _ => None,
            })
            .collect()
    }

    #[getter]
    /// Get all rules
    pub fn rules(&self) -> Vec<RuleDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Rule(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }
}
}

impl OntologyFile {
    pub fn concepts_as_ref(&self) -> Vec<&ConceptDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Concept(c) => Some(c),
                _ => None,
            })
            .collect()
    }

    /// Get all properties
    pub fn properties_as_ref(&self) -> Vec<&PropertyDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Property(p) => Some(p),
                _ => None,
            })
            .collect()
    }

    /// Get all rules
    pub fn rules_as_ref(&self) -> Vec<&RuleDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Rule(r) => Some(r),
                _ => None,
            })
            .collect()
    }
}
/// Package manifest file
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct PackageFile {
    /// Package name (qualified)
    pub name: QualifiedName,
    /// Dolfin language version
    pub dolfin_version: String,
    /// Package version
    pub version: String,
    /// Optional author
    pub author: Option<String>,
    /// Optional description
    pub description: Option<String>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl PackageFile {
    /// Create a new package file
    #[new]
    #[pyo3(signature = (name, dolfin_version, version, author=None, description=None, span=None))]
    pub fn new(
        name: QualifiedName,
        dolfin_version: String,
        version: String,
        author: Option<String>,
        description: Option<String>,
        span: Option<Span>,
    ) -> Self {
        Self { name, dolfin_version, version, author, description, span }
    }

    fn __repr__(&self) -> String {
        format!(
            "PackageFile({}, dolfin_version={}, version={})",
            self.name.full(),
            self.dolfin_version,
            self.version
        )
    }
}
}

/// Internal enum for parsing package fields
#[derive(Debug, Clone)]
pub enum PackageField {
    /// Dolfin language version specification
    DolfinVersion(String),
    /// Package version
    Version(String),
    /// Package author
    Author(String),
    /// Package description
    Description(String),
}

/// Prefix declaration
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct PrefixDecl {
    /// The path being aliased
    pub path: QualifiedName,
    /// The alias (short name)
    pub alias: String,
    /// Source code span for this declaration
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl PrefixDecl {
    /// Create a new prefix declaration
    #[new]
    pub fn new(path: QualifiedName, alias: String, span: Option<Span>) -> Self {
        Self { path, alias, span }
    }

    fn __repr__(&self) -> String {
        format!("PrefixDecl({} as {})", self.path.full(), self.alias)
    }
}
}

impl PrefixDecl {
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Members of an ontology
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Declaration {
    /// A concept definition
    Concept(ConceptDef),
    /// A property definition
    Property(PropertyDef),
    /// A rule definition
    Rule(RuleDef),
}

impl_python! {
#[pymethods]
impl Declaration {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Declaration::Concept { .. } => "concept",
            Declaration::Property { .. } => "property",
            Declaration::Rule { .. } => "rule",
        }
    }

    #[getter]
    fn concept(&self) -> Option<ConceptDef> {
        match self {
            Declaration::Concept(def) => Some(def.clone()),
            _ => None,
        }
    }

    #[getter]
    fn property(&self) -> Option<PropertyDef> {
        match self {
            Declaration::Property(def) => Some(def.clone()),
            _ => None,
        }
    }

    #[getter]
    fn rule(&self) -> Option<RuleDef> {
        match self {
            Declaration::Rule(def) => Some(def.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Declaration::Concept(def) => format!("Declaration::Concept('{}')", def.name.get()),
            Declaration::Property(def) => format!("Declaration::Property('{}')", def.name.get()),
            Declaration::Rule(def) => format!("Declaration::Rule('{}')", def.name),
        }
    }

    #[getter]
    pub fn name(&self) -> String {
        match self {
            Declaration::Concept(c) => c.name.get().clone(),
            Declaration::Property(p) => p.name.get().clone(),
            Declaration::Rule(r) => r.name.clone(),
        }
    }
}
}

/// Concept definition
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConceptDef {
    /// The concept name (with optional name-token span)
    pub name: SpannedString,
    /// Parent types this concept inherits from
    pub parents: Vec<TypeRef>,
    /// Property declarations (has statements)
    pub has_declarations: Vec<HasDeclaration>,
    /// Named individuals declared with 'one of:' (closed-world enumeration)
    pub one_of: Option<Vec<OneOfVariant>>,
    /// Source code span for this definition
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ConceptDef {
    #[new]
    #[pyo3(signature = (name, parents, has_declarations, one_of, span, name_span=None))]
    pub fn new(name: String, parents: Vec<TypeRef>, has_declarations: Vec<HasDeclaration>, one_of: Option<Vec<OneOfVariant>>, span: Option<Span>, name_span: Option<Span>) -> Self {
        Self { name: SpannedString::new(name, name_span), parents, has_declarations, one_of, span }
    }

    fn __repr__(&self) -> String {
        format!(
            "ConceptDef('{}', declarations={})",
            self.name.get(),
            self.has_declarations.len()
        )
    }
}
}

impl ConceptDef {
    /// Constructor without span - for tests and programmatic creation.
    pub fn without_span(
        name: String,
        parents: Vec<TypeRef>,
        has_declarations: Vec<HasDeclaration>,
    ) -> Self {
        Self {
            name: SpannedString::new(name, None),
            parents,
            has_declarations,
            one_of: None,
            span: None,
        }
    }

    /// Attach a span to this node (builder pattern).
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

/// Declarations within a concept
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum ConceptMember {
    /// Subclass declarations (parent types)
    Sub(Vec<TypeRef>),
    /// Property declarations
    Has(HasDeclaration),
    /// Closed-world named individuals ('one of:' block)
    OneOf(Vec<OneOfVariant>),
}

impl_python! {
#[pymethods]
impl ConceptMember {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            ConceptMember::Sub(..) => "sub",
            ConceptMember::Has(..) => "has",
            ConceptMember::OneOf(..) => "one_of",
        }
    }

    #[getter]
    fn sub_types(&self) -> Option<Vec<TypeRef>> {
        match self {
            ConceptMember::Sub(types) => Some(types.clone()),
            _ => None,
        }
    }

    #[getter]
    fn has(&self) -> Option<HasDeclaration> {
        match self {
            ConceptMember::Has(decl) => Some(decl.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            ConceptMember::Sub(types) => {
                format!("ConceptMember::Sub(count={})", types.len())
            }
            ConceptMember::Has(decl) => format!("ConceptMember::Has('{}')", decl.name),
            ConceptMember::OneOf(variants) => format!("ConceptMember::OneOf(count={})", variants.len()),
        }
    }
}
}
/// Has declaration (property on a concept)
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct HasDeclaration {
    /// The property name
    pub name: String,
    /// Whether this property is part of the primary key set
    pub is_key: bool,
    /// Optional cardinality constraint
    pub cardinality: Option<Cardinality>,
    /// The type reference for this property
    pub type_ref: TypeRef,
    /// Source code span for this declaration
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl HasDeclaration {
    /// Create a new HasDeclaration.
    ///
    /// Args:
    ///     name: The property name
    ///     type_ref: The type reference
    ///     cardinality: Optional cardinality constraint (default: None)
    #[new]
    #[pyo3(signature = (name, type_ref, cardinality=None, is_key=false, span=None))]
    pub fn new(name: String, type_ref: TypeRef, cardinality: Option<&Cardinality>, is_key: bool, span: Option<Span>) -> Self {
        Self {
            name,
            is_key,
            cardinality: cardinality.cloned(),
            type_ref,
            span,
        }
    }

    fn __repr__(&self) -> String {
        let key = if self.is_key { "key " } else { "" };
        match &self.cardinality {
            Some(card) => format!("HasDeclaration({}{}:  {} {})", key, self.name, card, self.type_ref),
            None => format!("HasDeclaration({}{}:  {})", key, self.name, self.type_ref),
        }
    }
}
}

/// Property definition
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct PropertyDef {
    /// The property name
    pub name: SpannedString,
    /// Optional cardinality constraint for the domain
    pub domain_cardinality: Option<Cardinality>,
    /// The domain type (subject type)
    pub domain: TypeRef,
    /// Optional cardinality constraint for the range
    pub range_cardinality: Option<Cardinality>,
    /// The range type (object type)
    pub range: TypeRef,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl PropertyDef {
    /// Create a new PropertyDef.
    ///
    /// Args:
    ///     name: The property name
    ///     domain: The domain type
    ///     range: The range type
    ///     domain_cardinality: Optional domain cardinality (default: None)
    ///     range_cardinality: Optional range cardinality (default: None)
    #[new]
    #[pyo3(signature = (name, domain, range, domain_cardinality=None, range_cardinality=None, span=None, name_span=None))]
    pub fn new(
        name: String,
        domain: TypeRef,
        range: TypeRef,
        domain_cardinality: Option<&Cardinality>,
        range_cardinality: Option<&Cardinality>,
        span: Option<Span>,
        name_span: Option<Span>,
    ) -> Self {
        Self {
            name: SpannedString::new(name, name_span),
            domain_cardinality: domain_cardinality.cloned(),
            domain,
            range_cardinality: range_cardinality.cloned(),
            range,
            span,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "PropertyDef({}: {} -> {})",
            self.name.get(), self.domain, self.range
        )
    }
}
}

/// A named individual declared inside a concept's 'one of:' block.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct OneOfVariant {
    /// The individual name (e.g. WHITE, RED)
    pub name: String,
    /// Optional constraint block fixing key property values
    pub constraints: Option<ConstraintBlock>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl OneOfVariant {
    #[new]
    #[pyo3(signature = (name, constraints=None, span=None))]
    pub fn new(name: String, constraints: Option<ConstraintBlock>, span: Option<Span>) -> Self {
        Self { name, constraints, span }
    }

    fn __repr__(&self) -> String {
        format!("OneOfVariant('{}')", self.name)
    }
}
}

/// Rule definition with match/then blocks
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct RuleDef {
    /// The rule name
    pub name: String,
    /// The match block containing patterns
    pub match_block: MatchBlock,
    /// The then block containing assertions
    pub then_block: ThenBlock,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl RuleDef {
    #[new]
    pub fn new(name: String, match_block: MatchBlock, then_block: ThenBlock, span: Option<Span>) -> Self {
        Self {
            name,
            match_block,
            then_block,
            span,
        }
    }

    fn __repr__(&self) -> String {
        format!("RuleDef('{}')", self.name)
    }
}
}

/// Match block containing patterns
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct MatchBlock {
    /// The patterns to match
    pub patterns: Vec<Pattern>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl MatchBlock {
    #[new]
    pub fn new(patterns: Vec<Pattern>, span: Option<Span>) -> Self {
        Self { patterns, span }
    }

    fn __repr__(&self) -> String {
        format!("MatchBlock(patterns={})", self.patterns.len())
    }

    fn __len__(&self) -> usize {
        self.patterns.len()
    }
}
}
/// Then block containing assertions and nested rules
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ThenBlock {
    /// The items to execute when patterns match
    pub items: Vec<ThenItem>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ThenBlock {
    #[new]
    pub fn new(items: Vec<ThenItem>, span: Option<Span>) -> Self {
        Self { items, span }
    }

    fn __repr__(&self) -> String {
        format!("ThenBlock(items={})", self.items.len())
    }

    fn __len__(&self) -> usize {
        self.items.len()
    }
}
}
/// Items in a then block
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum ThenItem {
    /// An assertion triple (subject-property-object)
    AssertionTriple {
        /// The assertion to make
        assertion: Assertion,
        span: Option<Span>,
    },
    /// A type assertion (subject is of type)
    AssertionTyping {
        /// The subject being typed
        subject: Subject,
        /// The type to assert
        typing: QualifiedName,
        span: Option<Span>,
    },
    /// A nested rule definition
    NestedRule {
        /// The nested rule
        rule: RuleDef,
    },
}

impl_python! {
#[pymethods]
impl ThenItem {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            ThenItem::AssertionTriple { .. } => "assertion",
            ThenItem::AssertionTyping { .. } => "typing",
            ThenItem::NestedRule { .. } => "nested_rule",
        }
    }

    #[getter]
    fn assertion(&self) -> Option<Assertion> {
        match self {
            ThenItem::AssertionTriple { assertion, .. } => Some(assertion.clone()),
            _ => None,
        }
    }

    #[getter]
    fn nested_rule(&self) -> Option<RuleDef> {
        match self {
            ThenItem::NestedRule { rule } => Some(rule.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            ThenItem::AssertionTriple { .. } => "ThenItem::Assertion".to_string(),
            ThenItem::AssertionTyping { .. } => "ThenItem::Typing".to_string(),
            ThenItem::NestedRule { .. } => "ThenItem::NestedRule".to_string(),
        }
    }
}
}
/// Assertion in a then block
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct Assertion {
    /// The subject of the assertion
    pub subject: Subject,
    /// The property being asserted
    pub property: QualifiedName,
    /// The object/value of the assertion
    pub object: Object,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl Assertion {
    #[new]
    pub fn new(subject: Subject, property: QualifiedName, object: Object, span: Option<Span>) -> Self {
        Self {
            subject,
            property,
            object,
            span,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Assertion({:?} {} {:?})",
            self.subject, self.property, self.object
        )
    }
}
}
/// Pattern in a match block
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// A triple pattern (subject-property-object)
    Triple {
        /// The subject to match
        subject: Subject,
        /// The property to match
        property: QualifiedName,
        /// The object to match
        object: Object,
        span: Option<Span>,
    },
    /// A type pattern (subject is of type)
    Type {
        /// The subject to check
        subject: Subject,
        /// The type to match
        type_ref: TypeRef,
        span: Option<Span>,
    },
    /// A quantified pattern (all/any/some/none with nested patterns)
    Quantified {
        /// The quantifier (all, any, some, etc.)
        quantifier: Quantifier,
        /// The variable being quantified
        variable: String,
        /// Optional constraints on the variable
        constraint: Option<ConstraintBlock>,
        /// Nested patterns to match
        patterns: Vec<Pattern>,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Pattern {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Pattern::Triple { .. } => "triple",
            Pattern::Type { .. } => "type",
            Pattern::Quantified { .. } => "quantified",
        }
    }

    #[getter]
    fn subject(&self) -> Option<Subject> {
        match self {
            Pattern::Triple { subject, .. } | Pattern::Type { subject, .. } => {
                Some(subject.clone())
            }
            Pattern::Quantified { .. } => None,
        }
    }

    #[getter]
    fn property(&self) -> Option<QualifiedName> {
        match self {
            Pattern::Triple { property, .. } => Some(property.clone()),
            _ => None,
        }
    }

    #[getter]
    fn object(&self) -> Option<Object> {
        match self {
            Pattern::Triple { object, .. } => Some(object.clone()),
            _ => None,
        }
    }

    #[getter]
    fn type_ref(&self) -> Option<TypeRef> {
        match self {
            Pattern::Type { type_ref, .. } => Some(type_ref.clone()),
            _ => None,
        }
    }

    #[getter]
    fn quantifier(&self) -> Option<Quantifier> {
        match self {
            Pattern::Quantified { quantifier, .. } => Some(quantifier.clone()),
            _ => None,
        }
    }

    #[getter]
    fn variable(&self) -> Option<String> {
        match self {
            Pattern::Quantified { variable, .. } => Some(variable.clone()),
            _ => None,
        }
    }

    #[getter]
    fn constraint(&self) -> Option<ConstraintBlock> {
        match self {
            Pattern::Quantified { constraint, .. } => constraint.clone(),
            _ => None,
        }
    }

    #[getter]
    fn patterns(&self) -> Option<Vec<Pattern>> {
        match self {
            Pattern::Quantified { patterns, .. } => Some(patterns.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Pattern::Triple {
                subject,
                property,
                object,
                ..
            } => {
                format!("Pattern::Triple({:?} {} {:?})", subject, property, object)
            }
            Pattern::Type { subject, type_ref, .. } => {
                format!("Pattern::Type({:?} is {:?})", subject, type_ref)
            }
            Pattern::Quantified {
                quantifier,
                variable,
                ..
            } => {
                format!("Pattern::Quantified({:?} {})", quantifier, variable)
            }
        }
    }
}
}
/// Subject of a pattern
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Subject {
    /// A variable (e.g., ?x)
    Variable { name: String, span: Option<Span> },
    /// A constant qualified name
    Constant {
        name: QualifiedName,
        span: Option<Span>,
    },
    /// A constrained subject
    Constraint {
        block: ConstraintBlock,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Subject {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Subject::Variable { .. } => "variable",
            Subject::Constraint { .. } => "constraint",
            Subject::Constant { .. } => "constant",
        }
    }

    #[getter]
    fn variable(&self) -> Option<String> {
        match self {
            Subject::Variable { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn constraint(&self) -> Option<ConstraintBlock> {
        match self {
            Subject::Constraint { block, .. } => Some(block.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Subject::Variable { name, .. } => format!("Subject::Variable('{}')", name),
            Subject::Constraint { .. } => "Subject::Constraint(...)".to_string(),
            Subject::Constant { name, .. } => format!("Subject::Constant('{}')", name),
        }
    }
}
}
/// Binary arithmetic operator
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

impl_python! {
#[pymethods]
impl BinaryOp {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            BinaryOp::Add => "add",
            BinaryOp::Sub => "sub",
            BinaryOp::Mul => "mul",
            BinaryOp::Div => "div",
        }
    }

    fn __repr__(&self) -> String {
        format!("BinaryOp::{:?}", self)
    }
}
}

/// Unary arithmetic operator
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,
}

impl_python! {
#[pymethods]
impl UnaryOp {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            UnaryOp::Neg => "neg",
        }
    }

    fn __repr__(&self) -> String {
        "UnaryOp::Neg".to_string()
    }
}
}

/// Mathematical expression (used wherever a value is expected)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: Literal,
        span: Option<Span>,
    },
    BinaryOp {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Option<Span>,
    },
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Expr {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Expr::Literal { .. } => "literal",
            Expr::BinaryOp { .. } => "binary_op",
            Expr::UnaryOp { .. } => "unary_op",
        }
    }

    #[getter]
    fn literal(&self) -> Option<Literal> {
        match self {
            Expr::Literal { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn op_binary(&self) -> Option<BinaryOp> {
        match self {
            Expr::BinaryOp { op, .. } => Some(*op),
            _ => None,
        }
    }

    #[getter]
    fn op_unary(&self) -> Option<UnaryOp> {
        match self {
            Expr::UnaryOp { op, .. } => Some(*op),
            _ => None,
        }
    }

    #[getter]
    fn left(&self) -> Option<Expr> {
        match self {
            Expr::BinaryOp { left, .. } => Some(*left.clone()),
            _ => None,
        }
    }

    #[getter]
    fn right(&self) -> Option<Expr> {
        match self {
            Expr::BinaryOp { right, .. } => Some(*right.clone()),
            _ => None,
        }
    }

    #[getter]
    fn operand(&self) -> Option<Expr> {
        match self {
            Expr::UnaryOp { operand, .. } => Some(*operand.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Expr::Literal { value, .. } => format!("Expr::Literal({:?})", value),
            Expr::BinaryOp { op, left, right, .. } => {
                format!("Expr::BinaryOp({:?}, {:?}, {:?})", op, left, right)
            }
            Expr::UnaryOp { op, operand, .. } => {
                format!("Expr::UnaryOp({:?}, {:?})", op, operand)
            }
        }
    }
}
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Literal { value, .. } => write!(f, "{}", value),
            Expr::BinaryOp { op, left, right, .. } => {
                let sym = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                };
                write!(f, "({} {} {})", left, sym, right)
            }
            Expr::UnaryOp { op, operand, .. } => {
                let sym = match op {
                    UnaryOp::Neg => "-",
                };
                write!(f, "({}{})", sym, operand)
            }
        }
    }
}

use crate::macros::py_only;

py_only! {
    impl<'py> pyo3::IntoPyObject<'py> for Box<Expr> {
        type Target = Expr;
        type Output = pyo3::Bound<'py, Expr>;
        type Error = pyo3::PyErr;

        fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
            (*self).into_pyobject(py)
        }
    }

    impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for Box<Expr> {
        type Error = pyo3::PyErr;
        fn extract(ob: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
            ob.extract::<Expr>().map(Box::new).map_err(Into::into)
        }
    }
}

/// Object of a pattern
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Object {
    Variable {
        name: String,
        span: Option<Span>,
    },
    Literal {
        value: Expr,
        span: Option<Span>,
    },
    Constraint {
        block: ConstraintBlock,
    },
    Constant {
        value: QualifiedName,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Object {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Object::Variable { .. } => "variable",
            Object::Literal { .. } => "literal",
            Object::Constraint { .. } => "constraint",
            Object::Constant { .. } => "constant",
        }
    }

    #[getter]
    fn variable(&self) -> Option<String> {
        match self {
            Object::Variable { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn expr(&self) -> Option<Expr> {
        match self {
            Object::Literal { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn constraint(&self) -> Option<ConstraintBlock> {
        match self {
            Object::Constraint { block, .. } => Some(block.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Object::Variable { name, .. } => format!("Object::Variable('{}')", name),
            Object::Literal { value, .. } => format!("Object::Literal({:?})", value),
            Object::Constraint { .. } => "Object::Constraint(...)".to_string(),
            Object::Constant { value, .. } => format!("Object::Constraint('{}')", value),
        }
    }
}
}
/// Constraint block with conditions
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ConstraintBlock {
    pub constraints: Vec<Constraint>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ConstraintBlock {
    #[new]
    pub fn new(constraints: Vec<Constraint>, span: Option<Span>) -> Self {
        Self { constraints, span }
    }

    fn __repr__(&self) -> String {
        format!("ConstraintBlock(constraints={})", self.constraints.len())
    }

    fn __len__(&self) -> usize {
        self.constraints.len()
    }
}
}
/// Individual constraint
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    TypeIs {
        type_ref: TypeRef,
        span: Option<Span>,
    },
    Comparison {
        operator: ComparisonOp,
        value: Expr,
        span: Option<Span>,
    },
    PropertyValue {
        property: QualifiedName,
        value: Object,
        span: Option<Span>,
    },
    PropertyConstraint {
        property: QualifiedName,
        block: ConstraintBlock,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Constraint {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Constraint::TypeIs { .. } => "type_is",
            Constraint::Comparison { .. } => "comparison",
            Constraint::PropertyValue { .. } => "property_value",
            Constraint::PropertyConstraint { .. } => "property_constraint",
        }
    }

    #[getter]
    fn type_ref(&self) -> Option<TypeRef> {
        match self {
            Constraint::TypeIs { type_ref, .. } => Some(type_ref.clone()),
            _ => None,
        }
    }

    #[getter]
    fn operator(&self) -> Option<ComparisonOp> {
        match self {
            Constraint::Comparison { operator, .. } => Some(*operator),
            _ => None,
        }
    }

    #[getter]
    fn value(&self) -> Option<Expr> {
        match self {
            Constraint::Comparison { value, .. } => Some(value.clone()),
            Constraint::PropertyValue { value: Object::Literal { value, .. }, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn property(&self) -> Option<QualifiedName> {
        match self {
            Constraint::PropertyValue { property, .. } => Some(property.clone()),
            Constraint::PropertyConstraint { property, .. } => Some(property.clone()),
            _ => None,
        }
    }

    #[getter]
    fn block(&self) -> Option<ConstraintBlock> {
        match self {
            Constraint::PropertyConstraint { block, .. } => Some(block.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Constraint::TypeIs { type_ref, .. } => format!("Constraint::TypeIs({:?})", type_ref),
            Constraint::Comparison { operator, value, .. } => {
                format!("Constraint::Comparison({:?} {:?})", operator, value)
            }
            Constraint::PropertyValue { property, value, .. } => {
                format!("Constraint::PropertyValue({} {:?})", property, value)
            }
            Constraint::PropertyConstraint { property, block, .. } => {
                format!("Constraint::PropertyConstraint({} {:?})", property, block)
            }
        }
    }
}
}
/// Literal values
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int { value: i64, span: Option<Span> },
    Float { value: f64, span: Option<Span> },
    String { value: String, span: Option<Span> },
    Boolean { value: bool, span: Option<Span> },
    Iri { value: String, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl Literal {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Literal::Int { .. } => "int",
            Literal::Float { .. } => "float",
            Literal::String { .. } => "string",
            Literal::Boolean { .. } => "boolean",
            Literal::Iri { .. } => "iri",
        }
    }

    #[getter]
    fn int_value(&self) -> Option<i64> {
        match self {
            Literal::Int { value: v, .. } => Some(*v),
            _ => None,
        }
    }

    #[getter]
    fn float_value(&self) -> Option<f64> {
        match self {
            Literal::Float { value: v, .. } => Some(*v),
            _ => None,
        }
    }

    #[getter]
    fn string_value(&self) -> Option<String> {
        match self {
            Literal::String { value: v, .. } => Some(v.clone()),
            _ => None,
        }
    }

    #[getter]
    fn boolean_value(&self) -> Option<bool> {
        match self {
            Literal::Boolean { value: v, .. } => Some(*v),
            _ => None,
        }
    }

    #[getter]
    fn iri_value(&self) -> Option<String> {
        match self {
            Literal::Iri { value: v, .. } => Some(v.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Literal::Int { value: v, .. } => format!("Literal::Int({})", v),
            Literal::Float { value: v, .. } => format!("Literal::Float({})", v),
            Literal::String { value: v, .. } => format!("Literal::String({:?})", v),
            Literal::Boolean { value: v, .. } => format!("Literal::Boolean({})", v),
            Literal::Iri { value: v, .. } => format!("Literal::Iri({})", v),
        }
    }
}
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::Int { value: v, .. } => write!(f, "{}", v),
            Literal::Float { value: v, .. } => write!(f, "{}", v),
            Literal::String { value: v, .. } => write!(f, "{:?}", v),
            Literal::Boolean { value: v, .. } => write!(f, "{}", if *v { "true" } else { "false" }),
            Literal::Iri { value: v, .. } => write!(f, "<{}>", v),
        }
    }
}

/// Type reference (qualified name or primitive)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum TypeRef {
    Named {
        name: QualifiedName,
        span: Option<Span>,
    },
    Primitive {
        kind: PrimitiveKind,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl TypeRef {
    /// Create a named type reference
    #[staticmethod]
    fn named(name: QualifiedName) -> Self {
        TypeRef::Named { name, span: None }
    }

    /// Create a primitive type reference
    #[staticmethod]
    fn primitive(kind: PrimitiveKind) -> Self {
        TypeRef::Primitive { kind, span: None }
    }

    /// Create a string type reference
    #[staticmethod]
    fn string() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::String,
            span: None,
        }
    }

    /// Create an int type reference
    #[staticmethod]
    fn int() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Int,
            span: None,
        }
    }

    /// Create a float type reference
    #[staticmethod]
    fn float() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Float,
            span: None,
        }
    }

    /// Create a boolean type reference
    #[staticmethod]
    fn boolean() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Boolean,
            span: None,
        }
    }

    #[getter]
    fn type_kind(&self) -> &str {
        match self {
            TypeRef::Named { .. } => "named",
            TypeRef::Primitive { .. } => "primitive",
        }
    }

    #[getter]
    fn name(&self) -> Option<QualifiedName> {
        match self {
            TypeRef::Named { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn primitive_kind(&self) -> Option<PrimitiveKind> {
        match self {
            TypeRef::Primitive { kind, .. } => Some(*kind),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            TypeRef::Named { name, .. } => format!("TypeRef.Named({})", name),
            TypeRef::Primitive { kind, .. } => format!("TypeRef.Primitive({:?})", kind),
        }
    }

    fn __str__(&self) -> String {
        match self {
            TypeRef::Named { name, .. } => name.full(),
            TypeRef::Primitive { kind, .. } => format!("{:?}", kind).to_lowercase(),
        }
    }
}
}
impl fmt::Display for TypeRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeRef::Named { name, .. } => write!(f, "{}", name),
            TypeRef::Primitive { kind, .. } => write!(f, "{:?}", kind),
        }
    }
}
/// Cardinality constraint
#[cfg_attr(feature = "python", pyclass(frozen, skip_from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Cardinality {
    One {
        span: Option<Span>,
    },
    Any {
        span: Option<Span>,
    },
    Some {
        span: Option<Span>,
    },
    Optional {
        span: Option<Span>,
    },
    Exact {
        value: usize,
        span: Option<Span>,
    },
    Range {
        min: usize,
        max: Option<usize>,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Cardinality {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            Cardinality::One { .. } => "one",
            Cardinality::Any { .. } => "any",
            Cardinality::Some { .. } => "some",
            Cardinality::Optional { .. } => "optional",
            Cardinality::Exact { .. } => "exact",
            Cardinality::Range { .. } => "range",
        }
    }

    #[getter]
    fn exact(&self) -> Option<usize> {
        match self {
            Cardinality::Exact { value: n, .. } => Some(*n),
            _ => None,
        }
    }

    #[getter]
    fn min(&self) -> Option<usize> {
        match self {
            Cardinality::Range { min, .. } => Some(*min),
            _ => None,
        }
    }

    #[getter]
    fn max(&self) -> Option<usize> {
        match self {
            Cardinality::Range { max, .. } => *max,
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Cardinality::One { .. } => "Cardinality.One".to_string(),
            Cardinality::Any { .. } => "Cardinality.Any".to_string(),
            Cardinality::Some { .. } => "Cardinality.Some".to_string(),
            Cardinality::Optional { .. } => "Cardinality.Optional".to_string(),
            Cardinality::Exact { value: n, .. } => format!("Cardinality.Exact({})", n),
            Cardinality::Range { min, max, .. } => match max {
                Some(m) => format!("Cardinality.Range({}, {})", min, m),
                None => format!("Cardinality.Range({}, *)", min),
            },
        }
    }

    fn __str__(&self) -> String {
        match self {
            Cardinality::One { .. } => "one".to_string(),
            Cardinality::Any { .. } => "any".to_string(),
            Cardinality::Some { .. } => "some".to_string(),
            Cardinality::Optional { .. } => "optional".to_string(),
            Cardinality::Exact { value: n, .. } => format!("{}", n),
            Cardinality::Range { min, max, .. } => match max {
                Some(m) => format!("{}..{}", min, m),
                None => format!("{}..*", min),
            },
        }
    }
}
}
impl fmt::Display for Cardinality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.__str__())
    }
}

/// Cardinality value (for quantifiers)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum CardinalityValue {
    Int { value: usize, span: Option<Span> },
    Variable { name: String, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl CardinalityValue {
    /// Create an integer cardinality value
    #[staticmethod]
    fn from_int(n: usize) -> Self {
        CardinalityValue::Int { value: n, span: None }
    }

    /// Create a variable cardinality value
    #[staticmethod]
    fn from_variable(name: String) -> Self {
        CardinalityValue::Variable { name, span: None }
    }

    #[getter]
    fn kind(&self) -> &str {
        match self {
            CardinalityValue::Int { .. } => "int",
            CardinalityValue::Variable { .. } => "variable",
        }
    }

    #[getter]
    fn int_value(&self) -> Option<usize> {
        match self {
            CardinalityValue::Int { value: n, .. } => Some(*n),
            _ => None,
        }
    }

    #[getter]
    fn variable(&self) -> Option<String> {
        match self {
            CardinalityValue::Variable { name: v, .. } => Some(v.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            CardinalityValue::Int { value: n, .. } => format!("CardinalityValue.Int({})", n),
            CardinalityValue::Variable { name: v, .. } => format!("CardinalityValue.Variable({})", v),
        }
    }
}
}

/// Integer or variable reference (for cardinalities)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum IntOrVar {
    Int { value: usize },
    Variable { name: String },
}

impl_python! {
#[pymethods]
impl IntOrVar {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            IntOrVar::Int { .. } => "int",
            IntOrVar::Variable { .. } => "variable",
        }
    }

    #[getter]
    fn int_value(&self) -> Option<usize> {
        match self {
            IntOrVar::Int { value } => Some(*value),
            _ => None,
        }
    }

    #[getter]
    fn variable(&self) -> Option<String> {
        match self {
            IntOrVar::Variable { name } => Some(name.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            IntOrVar::Int { value } => format!("IntOrVar::Int({})", value),
            IntOrVar::Variable { name } => format!("IntOrVar::Variable('{}')", name),
        }
    }
}
}
/// Quantifier types
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum Quantifier {
    All {
        span: Option<Span>,
    },
    None {
        span: Option<Span>,
    },
    Any {
        span: Option<Span>,
    },
    AtLeast {
        value: CardinalityValue,
        span: Option<Span>,
    },
    AtMost {
        value: CardinalityValue,
        span: Option<Span>,
    },
    Exactly {
        value: CardinalityValue,
        span: Option<Span>,
    },
    Between {
        min: CardinalityValue,
        max: CardinalityValue,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl Quantifier {
    /// Create an All quantifier
    #[staticmethod]
    fn all() -> Self {
        Quantifier::All { span: None }
    }

    /// Create a None quantifier
    #[staticmethod]
    fn none() -> Self {
        Quantifier::None { span: None }
    }

    /// Create an Any quantifier (existential)
    #[staticmethod]
    fn any() -> Self {
        Quantifier::Any { span: None }
    }

    /// Create an AtLeast quantifier
    #[staticmethod]
    fn at_least(value: CardinalityValue) -> Self {
        Quantifier::AtLeast { value, span: None }
    }

    /// Create an AtMost quantifier
    #[staticmethod]
    fn at_most(value: CardinalityValue) -> Self {
        Quantifier::AtMost { value, span: None }
    }

    /// Create an Exactly quantifier
    #[staticmethod]
    fn exactly(value: CardinalityValue) -> Self {
        Quantifier::Exactly { value, span: None }
    }

    /// Create a Between quantifier
    #[staticmethod]
    fn between(min: CardinalityValue, max: CardinalityValue) -> Self {
        Quantifier::Between { min, max, span: None }
    }

    #[getter]
    fn kind(&self) -> &str {
        match self {
            Quantifier::All { .. } => "all",
            Quantifier::None { .. } => "none",
            Quantifier::Any { .. } => "any",
            Quantifier::AtLeast { .. } => "at_least",
            Quantifier::AtMost { .. } => "at_most",
            Quantifier::Exactly { .. } => "exactly",
            Quantifier::Between { .. } => "between",
        }
    }

    #[getter]
    fn value(&self) -> Option<CardinalityValue> {
        match self {
            Quantifier::AtLeast { value, .. } => Some(value.clone()),
            Quantifier::AtMost { value, .. } => Some(value.clone()),
            Quantifier::Exactly { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn min(&self) -> Option<CardinalityValue> {
        match self {
            Quantifier::Between { min, .. } => Some(min.clone()),
            _ => None,
        }
    }

    #[getter]
    fn max(&self) -> Option<CardinalityValue> {
        match self {
            Quantifier::Between { max, .. } => Some(max.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Quantifier::All { .. } => "Quantifier.All".to_string(),
            Quantifier::None { .. } => "Quantifier.None".to_string(),
            Quantifier::Any { .. } => "Quantifier.Any".to_string(),
            Quantifier::AtLeast { value, .. } => format!("Quantifier.AtLeast({:?})", value),
            Quantifier::AtMost { value, .. } => format!("Quantifier.AtMost({:?})", value),
            Quantifier::Exactly { value, .. } => format!("Quantifier.Exactly({:?})", value),
            Quantifier::Between { min, max, .. } => format!("Quantifier.Between({:?}, {:?})", min, max),
        }
    }

    fn __str__(&self) -> String {
        match self {
            Quantifier::All { .. } => "All".to_string(),
            Quantifier::None { .. } => "None".to_string(),
            Quantifier::Any { .. } => "Any".to_string(),
            Quantifier::AtLeast { value, .. } => format!("AtLeast({:?})", value),
            Quantifier::AtMost { value, .. } => format!("AtMost({:?})", value),
            Quantifier::Exactly { value, .. } => format!("Exactly({:?})", value),
            Quantifier::Between { min, max, .. } => format!("Between({:?}, {:?})", min, max),
        }
    }
}
}

impl fmt::Display for Quantifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.__str__())
    }
}

/// Comparison operators
#[cfg_attr(feature = "python", pyclass(frozen, eq, eq_int, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComparisonOp {
    Equal,
    NotEqual,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
}

impl_python! {
#[pymethods]
impl ComparisonOp {
    fn __repr__(&self) -> &str {
        match self {
            ComparisonOp::Equal => "ComparisonOp.Equal",
            ComparisonOp::NotEqual => "ComparisonOp.NotEqual",
            ComparisonOp::LessThan => "ComparisonOp.LessThan",
            ComparisonOp::LessEqual => "ComparisonOp.LessEqual",
            ComparisonOp::GreaterThan => "ComparisonOp.GreaterThan",
            ComparisonOp::GreaterEqual => "ComparisonOp.GreaterEqual",
        }
    }

    fn __str__(&self) -> &str {
        match self {
            ComparisonOp::Equal => "=",
            ComparisonOp::NotEqual => "!=",
            ComparisonOp::LessThan => "<",
            ComparisonOp::LessEqual => "<=",
            ComparisonOp::GreaterThan => ">",
            ComparisonOp::GreaterEqual => ">=",
        }
    }
}
}
/// Primitive type kinds
#[cfg_attr(feature = "python", pyclass(frozen, eq, eq_int, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrimitiveKind {
    String,
    Int,
    Float,
    Boolean,
}

impl_python! {
#[pymethods]
impl PrimitiveKind {
    fn __repr__(&self) -> &str {
        match self {
            PrimitiveKind::String => "PrimitiveKind.String",
            PrimitiveKind::Int => "PrimitiveKind.Int",
            PrimitiveKind::Float => "PrimitiveKind.Float",
            PrimitiveKind::Boolean => "PrimitiveKind.Boolean",
        }
    }

    fn __str__(&self) -> &str {
        match self {
            PrimitiveKind::String => "string",
            PrimitiveKind::Int => "int",
            PrimitiveKind::Float => "float",
            PrimitiveKind::Boolean => "boolean",
        }
    }
}
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.__str__())
    }
}
