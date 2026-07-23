//! Abstract Syntax Tree definitions for the Dolfin language.
//!
//! All AST nodes are exposed to Python via PyO3.

#[cfg(feature = "python")]
use pyo3::prelude::*;
use std::fmt;

use crate::{
    error::{Span, SpannedString},
    macros::impl_python,
};

/// A qualified (dot-separated) name in Dolfin.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct QualifiedName {
    /// The individual components of the qualified name (e.g., ["com", "example", "Person"])
    pub parts: Vec<String>,
    /// True when parsed from `alias:Local` syntax (PREFIXED_NAME token), false for dot-separated.
    /// This distinguishes `ex:Animal` (prefix alias expansion) from `ex.Animal` (namespace path).
    pub is_prefixed: bool,
    /// Source code span for this declaration
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl QualifiedName {
    #[new]
    #[pyo3(signature = (parts, span=None))]
    pub fn new(parts: Vec<String>, span: Option<Span>) -> Self {
        Self { parts, is_prefixed: false, span }
    }

    /// Create a prefixed-name reference (from `alias:Local` syntax).
    #[staticmethod]
    pub fn new_prefixed(alias: String, local: String, span: Option<Span>) -> Self {
        Self { parts: vec![alias, local], is_prefixed: true, span }
    }

    #[staticmethod]
    pub fn from_single(name: String) -> Self {
        Self { parts: vec![name], is_prefixed: false, span: None }
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
            QualifiedName { parts, is_prefixed: false, span: Some(ss.merge(os)) }
        } else {
            QualifiedName { parts, is_prefixed: false, span: None }
        }
    }

    /// Remove last part and give a QualifiedName
    pub fn prefix_as_qn(&self) -> Option<QualifiedName> {
      if self.parts.len() > 1 {
        Some(QualifiedName { parts: self.parts[self.parts.len() - 1..].to_vec(), is_prefixed: false, span: None })
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
    QualifiedName { parts, is_prefixed: false, span: None }
  }
}

impl fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.full())
    }
}


/// Value of an `@iri_name` annotation — distinguishes how to resolve the name.
#[derive(Debug, Clone, PartialEq)]
pub enum IriNameValue {
    /// Absolute IRI from `<...>` syntax — used verbatim, not appended to base IRI
    AbsoluteUri(String),
    /// Local segment from `"..."` syntax — appended to the namespace base path
    LocalSegment(String),
}

impl IriNameValue {
    pub fn as_str(&self) -> &str {
        match self {
            IriNameValue::AbsoluteUri(s) | IriNameValue::LocalSegment(s) => s.as_str(),
        }
    }
}

#[cfg(feature = "python")]
impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for IriNameValue {
    type Error = pyo3::PyErr;
    fn extract(ob: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
        let s: String = ob.extract()?;
        Ok(IriNameValue::LocalSegment(s))
    }
}

#[cfg(feature = "python")]
impl<'py> pyo3::IntoPyObject<'py> for IriNameValue {
    type Target = pyo3::types::PyString;
    type Output = pyo3::Bound<'py, pyo3::types::PyString>;
    type Error = std::convert::Infallible;
    fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(pyo3::types::PyString::new(py, self.as_str()))
    }
}

#[cfg(feature = "python")]
impl<'py> pyo3::IntoPyObject<'py> for &IriNameValue {
    type Target = pyo3::types::PyString;
    type Output = pyo3::Bound<'py, pyo3::types::PyString>;
    type Error = std::convert::Infallible;
    fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
        Ok(pyo3::types::PyString::new(py, self.as_str()))
    }
}

/// An ontology file (new top-level structure)
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct OntologyFile {
    /// Optional IRI name override from @iri_name annotation
    pub iri_name: Option<IriNameValue>,
    /// Prefix declarations
    pub prefixes: Vec<PrefixDecl>,
    /// Declarations (concepts, properties, enums, rules)
    pub declarations: Vec<Declaration>,
    /// Raw `@locale <arg>` directive argument (e.g. `"d/m/y"`), if present.
    pub locale: Option<String>,
    /// Raw `@timezone <arg>` directive argument (e.g. `"Europe/Brussels"`).
    pub timezone: Option<String>,
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
        Self {
            iri_name: iri_name.map(IriNameValue::LocalSegment),
            prefixes,
            declarations,
            locale: None,
            timezone: None,
            span,
        }
    }

    /// The raw `@locale` directive argument, if the file declared one.
    #[getter]
    fn locale_directive(&self) -> Option<String> {
        self.locale.clone()
    }

    /// The raw `@timezone` directive argument, if the file declared one.
    #[getter]
    fn timezone_directive(&self) -> Option<String> {
        self.timezone.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "OntologyFile(iri_name={:?}, prefixes={}, declarations={})",
            self.iri_name.as_ref().map(|v| v.as_str()),
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

    #[getter]
    /// Get all facts
    pub fn facts(&self) -> Vec<FactDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Fact(f) => Some(f.clone()),
                _ => None,
            })
            .collect()
    }

    #[getter]
    /// Get all queries
    pub fn queries(&self) -> Vec<QueryDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Query(q) => Some(q.clone()),
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

    /// Get all facts
    pub fn facts_as_ref(&self) -> Vec<&FactDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Fact(f) => Some(f),
                _ => None,
            })
            .collect()
    }

    /// Get all queries
    pub fn queries_as_ref(&self) -> Vec<&QueryDef> {
        self.declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Query(q) => Some(q),
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
    /// A fact (instance) definition
    Fact(FactDef),
    /// A query definition
    Query(QueryDef),
    /// A unit declaration (`unitdef EUR: scale 1.0`, `unitdef family vegetables`, ...)
    Unit(UnitDef),
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
            Declaration::Fact { .. } => "fact",
            Declaration::Query { .. } => "query",
            Declaration::Unit { .. } => "unit",
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

    #[getter]
    fn fact(&self) -> Option<FactDef> {
        match self {
            Declaration::Fact(def) => Some(def.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Declaration::Concept(def) => format!("Declaration::Concept('{}')", def.name.get()),
            Declaration::Property(def) => format!("Declaration::Property('{}')", def.name.get()),
            Declaration::Rule(def) => format!("Declaration::Rule('{}')", def.name),
            Declaration::Fact(def) => format!("Declaration::Fact('{}')", def.id),
            Declaration::Query(def) => format!("Declaration::Query('{}')", def.name),
            Declaration::Unit(def) => format!("Declaration::Unit('{}')", def.name.get()),
        }
    }

    #[getter]
    pub fn name(&self) -> String {
        match self {
            Declaration::Concept(c) => c.name.get().clone(),
            Declaration::Property(p) => p.name.get().clone(),
            Declaration::Rule(r) => r.name.clone(),
            Declaration::Fact(f) => f.id.clone(),
            Declaration::Query(q) => q.name.clone(),
            Declaration::Unit(u) => u.name.get().clone(),
        }
    }

    #[getter]
    fn query(&self) -> Option<QueryDef> {
        match self {
            Declaration::Query(def) => Some(def.clone()),
            _ => None,
        }
    }
}
}

/// A unit declaration (`unitdef EUR: scale 1.0`, `unitdef family vegetables`, ...).
///
/// See `dolfin-units::registry::UnitRegistry` for how these feed the
/// project-scoped unit registry, and TODO.md's "currency / custom units"
/// entry for the overall design (currency units are ordinary derived units on
/// a new `Dimensions.currency` axis; nominal units are dimensionless but
/// incommensurable outside an explicit `as <family>` widening cast).
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct UnitDef {
    /// The unit (or family) name.
    pub name: SpannedString,
    /// What kind of unit declaration this is.
    pub kind: UnitKind,
    /// Source code span for this definition.
    pub span: Option<Span>,
}

/// The right-hand side of a `unit` declaration.
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum UnitKind {
    /// `unitdef family <name>` — declares an abstract, dimensionless family that
    /// nominal units can belong to (e.g. `vegetables`). Not itself a usable
    /// unit in a `quantity(...)` literal.
    Family(),
    /// `unit <name>: nominal of <family> scale <factor>` — an incommensurable
    /// unit belonging to `family`, convertible into the family's base scale
    /// only via an explicit `as <family>` widening cast (never via plain `+`
    /// with a different unit of the same family, and never via `*`/`/`).
    Nominal { family: QualifiedName, scale: f64 },
    /// `unit <name>: scale <factor> <reference>` — an ordinary derived unit,
    /// same mechanism as declaring `km` from `m`: dimensions and further
    /// scale are inherited from `reference` (which must already resolve,
    /// builtin or previously declared). This is how a project adds a
    /// currency (e.g. `unit USD: scale 0.92 EUR`) or any other derived unit.
    Derived { scale: f64, reference: String },
}

impl_python! {
#[pymethods]
impl UnitDef {
    #[new]
    #[pyo3(signature = (name, kind, span, name_span=None))]
    pub fn new(name: String, kind: UnitKind, span: Option<Span>, name_span: Option<Span>) -> Self {
        Self { name: SpannedString::new(name, name_span), kind, span }
    }

    fn __repr__(&self) -> String {
        format!("UnitDef('{}')", self.name.get())
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
    /// Optional IRI override from @iri_name annotation inside the concept body
    pub iri_name: Option<IriNameValue>,
    /// Source code span for this definition
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ConceptDef {
    #[new]
    #[pyo3(signature = (name, parents, has_declarations, one_of, span, name_span=None))]
    pub fn new(name: String, parents: Vec<TypeRef>, has_declarations: Vec<HasDeclaration>, one_of: Option<Vec<OneOfVariant>>, span: Option<Span>, name_span: Option<Span>) -> Self {
        Self { name: SpannedString::new(name, name_span), parents, has_declarations, one_of, iri_name: None, span }
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
            iri_name: None,
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
    /// IRI override for this specific concept
    IriName(IriNameValue),
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
            ConceptMember::IriName(..) => "iri_name",
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
            ConceptMember::IriName(iri) => format!("ConceptMember::IriName('{}')", iri.as_str()),
        }
    }
}
}
/// OWL property path expression (used in `equivalent to` axioms)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyPath {
    /// Bare property name
    Name { name: QualifiedName, span: Option<Span> },
    /// Inverse: ^p
    Inverse { inner: Box<PropertyPath>, span: Option<Span> },
    /// Sequence: p / q  (property chain)
    Sequence { steps: Vec<PropertyPath>, span: Option<Span> },
    /// Alternative: p | q
    Alt { left: Box<PropertyPath>, right: Box<PropertyPath>, span: Option<Span> },
    /// One-or-more: p+
    OneOrMore { inner: Box<PropertyPath>, span: Option<Span> },
    /// Zero-or-more: p*
    ZeroOrMore { inner: Box<PropertyPath>, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl PropertyPath {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            PropertyPath::Name { .. } => "name",
            PropertyPath::Inverse { .. } => "inverse",
            PropertyPath::Sequence { .. } => "sequence",
            PropertyPath::Alt { .. } => "alt",
            PropertyPath::OneOrMore { .. } => "one_or_more",
            PropertyPath::ZeroOrMore { .. } => "zero_or_more",
        }
    }

    #[getter]
    fn name(&self) -> Option<QualifiedName> {
        match self {
            PropertyPath::Name { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn steps(&self) -> Option<Vec<PropertyPath>> {
        match self {
            PropertyPath::Sequence { steps, .. } => Some(steps.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            PropertyPath::Name { name, .. } => format!("PropertyPath::Name({})", name),
            PropertyPath::Inverse { inner, .. } => format!("PropertyPath::Inverse({:?})", inner),
            PropertyPath::Sequence { steps, .. } => format!("PropertyPath::Sequence(len={})", steps.len()),
            PropertyPath::Alt { left, right, .. } => format!("PropertyPath::Alt({:?} | {:?})", left, right),
            PropertyPath::OneOrMore { inner, .. } => format!("PropertyPath::OneOrMore({:?})", inner),
            PropertyPath::ZeroOrMore { inner, .. } => format!("PropertyPath::ZeroOrMore({:?})", inner),
        }
    }
}
}

py_only! {
    impl<'py> pyo3::IntoPyObject<'py> for Box<PropertyPath> {
        type Target = PropertyPath;
        type Output = pyo3::Bound<'py, PropertyPath>;
        type Error = pyo3::PyErr;

        fn into_pyobject(self, py: pyo3::Python<'py>) -> Result<Self::Output, Self::Error> {
            (*self).into_pyobject(py)
        }
    }

    impl<'a, 'py> pyo3::FromPyObject<'a, 'py> for Box<PropertyPath> {
        type Error = pyo3::PyErr;
        fn extract(ob: pyo3::Borrowed<'a, 'py, pyo3::PyAny>) -> Result<Self, Self::Error> {
            ob.extract::<PropertyPath>().map(Box::new).map_err(Into::into)
        }
    }
}

/// Characteristic of a property (used in property definitions and has-declarations)
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyAxiom {
    Sub { property: QualifiedName, span: Option<Span> },
    InverseOf { property: QualifiedName, span: Option<Span> },
    Transitive { span: Option<Span> },
    Symmetric { span: Option<Span> },
    Reflexive { span: Option<Span> },
    EquivalentTo { path: PropertyPath, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl PropertyAxiom {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            PropertyAxiom::Sub { .. } => "sub",
            PropertyAxiom::InverseOf { .. } => "inverse_of",
            PropertyAxiom::Transitive { .. } => "transitive",
            PropertyAxiom::Symmetric { .. } => "symmetric",
            PropertyAxiom::Reflexive { .. } => "reflexive",
            PropertyAxiom::EquivalentTo { .. } => "equivalent_to",
        }
    }

    #[getter]
    fn property(&self) -> Option<QualifiedName> {
        match self {
            PropertyAxiom::Sub { property, .. } | PropertyAxiom::InverseOf { property, .. } => Some(property.clone()),
            _ => None,
        }
    }

    #[getter]
    fn path(&self) -> Option<PropertyPath> {
        match self {
            PropertyAxiom::EquivalentTo { path, .. } => Some(path.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            PropertyAxiom::Sub { property, .. } => format!("PropertyAxiom::Sub({})", property),
            PropertyAxiom::InverseOf { property, .. } => format!("PropertyAxiom::InverseOf({})", property),
            PropertyAxiom::Transitive { .. } => "PropertyAxiom::Transitive".to_string(),
            PropertyAxiom::Symmetric { .. } => "PropertyAxiom::Symmetric".to_string(),
            PropertyAxiom::Reflexive { .. } => "PropertyAxiom::Reflexive".to_string(),
            PropertyAxiom::EquivalentTo { path, .. } => format!("PropertyAxiom::EquivalentTo({:?})", path),
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
    /// Property axioms (inverse of, sub, etc.)
    pub axioms: Vec<PropertyAxiom>,
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
    #[pyo3(signature = (name, type_ref, cardinality=None, is_key=false, axioms=vec![], span=None))]
    pub fn new(name: String, type_ref: TypeRef, cardinality: Option<&Cardinality>, is_key: bool, axioms: Vec<PropertyAxiom>, span: Option<Span>) -> Self {
        Self {
            name,
            is_key,
            cardinality: cardinality.cloned(),
            type_ref,
            axioms,
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
    /// Property axioms (sub, inverse of, symmetric, etc.)
    pub axioms: Vec<PropertyAxiom>,
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
    #[pyo3(signature = (name, domain, range, domain_cardinality=None, range_cardinality=None, axioms=vec![], span=None, name_span=None))]
    pub fn new(
        name: String,
        domain: TypeRef,
        range: TypeRef,
        domain_cardinality: Option<&Cardinality>,
        range_cardinality: Option<&Cardinality>,
        axioms: Vec<PropertyAxiom>,
        span: Option<Span>,
        name_span: Option<Span>,
    ) -> Self {
        Self {
            name: SpannedString::new(name, name_span),
            domain_cardinality: domain_cardinality.cloned(),
            domain,
            range_cardinality: range_cardinality.cloned(),
            range,
            axioms,
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

/// Query definition
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct QueryDef {
    /// The query name
    pub name: String,
    /// The query body (clauses, optional group-by, optional return block)
    pub body: QueryBody,
    pub span: Option<Span>,
    /// Span of just the name token (for rename support)
    pub name_span: Option<Span>,
}

impl_python! {
#[pymethods]
impl QueryDef {
    #[new]
    pub fn new(name: String, body: QueryBody, span: Option<Span>) -> Self {
        Self { name, body, span, name_span: None }
    }

    fn __repr__(&self) -> String {
        format!("QueryDef('{}')", self.name)
    }
}
}

/// Body of a query definition
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct QueryBody {
    pub clauses: Vec<QueryClause>,
    pub group_by: Option<GroupByBlock>,
    pub return_block: Option<ReturnBlock>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl QueryBody {
    #[new]
    pub fn new(clauses: Vec<QueryClause>, group_by: Option<GroupByBlock>, return_block: Option<ReturnBlock>, span: Option<Span>) -> Self {
        Self { clauses, group_by, return_block, span }
    }
    fn __repr__(&self) -> String {
        format!("QueryBody(clauses={})", self.clauses.len())
    }
}
}

/// A top-level clause inside a query body
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum QueryClause {
    SubjectPattern(SubjectPattern),
    Composition(QueryComposition),
    ExistenceBlock(ExistenceBlock),
    InverseTriple(InverseTriple),
    BooleanFilter(BoolExpr),
    AggregationQuery(AggregationQuery),
}

impl_python! {
#[pymethods]
impl QueryClause {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            QueryClause::SubjectPattern(_) => "subject_pattern",
            QueryClause::Composition(_) => "composition",
            QueryClause::ExistenceBlock(_) => "existence_block",
            QueryClause::InverseTriple(_) => "inverse_triple",
            QueryClause::BooleanFilter(_) => "boolean_filter",
            QueryClause::AggregationQuery(_) => "aggregation_query",
        }
    }
    fn __repr__(&self) -> String {
        format!("QueryClause::{}", self.kind())
    }
}
}

/// A top-level inverse triple inside a query body: `is prop of ?var`
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct InverseTriple {
    pub property: QualifiedName,
    pub object: Object,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl InverseTriple {
    #[new]
    pub fn new(property: QualifiedName, object: Object, span: Option<Span>) -> Self {
        Self { property, object, span }
    }
    fn __repr__(&self) -> String {
        format!("InverseTriple({})", self.property)
    }
}
}

/// A subject-scoped pattern: `?var a Type { ... }`, `a Type { ... }` (anonymous), or `?var { ... }` (untyped)
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct SubjectPattern {
    pub subject: Option<String>,
    pub type_ref: Option<TypeRef>,
    pub properties: Vec<PropertyPattern>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl SubjectPattern {
    #[new]
    pub fn new(subject: Option<String>, type_ref: Option<TypeRef>, properties: Vec<PropertyPattern>, span: Option<Span>) -> Self {
        Self { subject, type_ref, properties, span }
    }
    fn __repr__(&self) -> String {
        format!("SubjectPattern({:?})", self.subject)
    }
}
}

/// A property-level pattern inside a subject block
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyPattern {
    Value        { property: QualifiedName, object: Object, span: Option<Span> },
    Constrained  { property: QualifiedName, block: ConstraintBlock, span: Option<Span> },
    Optional     { property: QualifiedName, object: Object, span: Option<Span> },
    Inverse        { property: QualifiedName, outer_var: String, span: Option<Span> },
    InverseNested  { property: QualifiedName, block: ConstraintBlock, span: Option<Span> },
    Nested         { property: QualifiedName, block: SubjectBlock, span: Option<Span> },
    Disjunction  { either_branch: DisjBranch, or_branches: Vec<DisjBranch>, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl PropertyPattern {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            PropertyPattern::Value { .. }          => "value",
            PropertyPattern::Constrained { .. }    => "constrained",
            PropertyPattern::Optional { .. }       => "optional",
            PropertyPattern::Inverse { .. }        => "inverse",
            PropertyPattern::InverseNested { .. }  => "inverse_nested",
            PropertyPattern::Nested { .. }         => "nested",
            PropertyPattern::Disjunction { .. }    => "disjunction",
        }
    }
    fn __repr__(&self) -> String {
        format!("PropertyPattern::{}", self.kind())
    }
}
}

/// An anonymous nested block: `[ property1 val\n  property2 val ]`
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct SubjectBlock {
    pub properties: Vec<PropertyPattern>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl SubjectBlock {
    #[new]
    pub fn new(properties: Vec<PropertyPattern>, span: Option<Span>) -> Self {
        Self { properties, span }
    }
    fn __repr__(&self) -> String {
        format!("SubjectBlock({})", self.properties.len())
    }
}
}

/// One branch in an `either … or …` disjunction
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct DisjBranch {
    pub property: QualifiedName,
    pub block: ConstraintBlock,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl DisjBranch {
    #[new]
    pub fn new(property: QualifiedName, block: ConstraintBlock, span: Option<Span>) -> Self {
        Self { property, block, span }
    }
    fn __repr__(&self) -> String {
        format!("DisjBranch({})", self.property)
    }
}
}

/// `some:` / `none:` existence filter block
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ExistenceBlock {
    /// `true` = `none:` (NOT EXISTS), `false` = `some:` (EXISTS)
    pub negated: bool,
    pub clauses: Vec<QueryClause>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ExistenceBlock {
    #[new]
    pub fn new(negated: bool, clauses: Vec<QueryClause>, span: Option<Span>) -> Self {
        Self { negated, clauses, span }
    }
    fn __repr__(&self) -> String {
        format!("ExistenceBlock(negated={})", self.negated)
    }
}
}

/// Body-level aggregation sub-query: `average ?x as ?y` with indented sub-patterns.
/// Generates an inner sub-SELECT with implicit GROUP BY.
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct AggregationQuery {
    pub kind: AggKind,
    pub input_var: String,
    pub result_var: String,
    pub sub_clauses: Vec<QueryClause>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl AggregationQuery {
    #[new]
    pub fn new(kind: AggKind, input_var: String, result_var: String, sub_clauses: Vec<QueryClause>, span: Option<Span>) -> Self {
        Self { kind, input_var, result_var, sub_clauses, span }
    }
    fn __repr__(&self) -> String {
        format!("AggregationQuery({:?} {} as {})", self.kind, self.input_var, self.result_var)
    }
}
}

/// Inline named-query composition: `queryName as [field ?var …]` or `queryName as ?var`
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct QueryComposition {
    pub query_name: QualifiedName,
    pub binding: CompositionBinding,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl QueryComposition {
    #[new]
    pub fn new(query_name: QualifiedName, binding: CompositionBinding, span: Option<Span>) -> Self {
        Self { query_name, binding, span }
    }
    fn __repr__(&self) -> String {
        format!("QueryComposition({})", self.query_name)
    }
}
}

/// How query results are bound at the call site
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum CompositionBinding {
    /// `[ fieldName ?localVar … ]` — rename projected columns
    Named(Vec<(String, String)>),
    /// `as ?var` — scalar single-result binding
    Scalar(String),
}

impl_python! {
#[pymethods]
impl CompositionBinding {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            CompositionBinding::Named(_) => "named",
            CompositionBinding::Scalar(_) => "scalar",
        }
    }
    fn __repr__(&self) -> String {
        format!("CompositionBinding::{}", self.kind())
    }
}
}

/// `group by ?var` block with aggregation specs and optional HAVING
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct GroupByBlock {
    pub var: String,
    pub specs: Vec<AggregationSpec>,
    pub having: Vec<BoolExpr>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl GroupByBlock {
    #[new]
    pub fn new(var: String, specs: Vec<AggregationSpec>, having: Vec<BoolExpr>, span: Option<Span>) -> Self {
        Self { var, specs, having, span }
    }
    fn __repr__(&self) -> String {
        format!("GroupByBlock({})", self.var)
    }
}
}

/// One aggregation line inside a `group by` block
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct AggregationSpec {
    pub kind: AggKind,
    pub input_var: String,
    pub result_var: String,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl AggregationSpec {
    #[new]
    pub fn new(kind: AggKind, input_var: String, result_var: String, span: Option<Span>) -> Self {
        Self { kind, input_var, result_var, span }
    }
    fn __repr__(&self) -> String {
        format!("AggregationSpec({:?} {} as {})", self.kind, self.input_var, self.result_var)
    }
}
}

/// Aggregation function kind
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggKind {
    Count,
    Average,
    Sum,
    Min,
    Max,
}

impl_python! {
#[pymethods]
impl AggKind {
    fn __repr__(&self) -> &str {
        match self {
            AggKind::Count   => "AggKind::Count",
            AggKind::Average => "AggKind::Average",
            AggKind::Sum     => "AggKind::Sum",
            AggKind::Min     => "AggKind::Min",
            AggKind::Max     => "AggKind::Max",
        }
    }
}
}

/// `return` block listing projected columns
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ReturnBlock {
    pub columns: Vec<ColumnSpec>,
    pub limit: Option<u64>,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ReturnBlock {
    #[new]
    pub fn new(columns: Vec<ColumnSpec>, limit: Option<u64>, span: Option<Span>) -> Self {
        Self { columns, limit, span }
    }
    fn __repr__(&self) -> String {
        format!("ReturnBlock(columns={})", self.columns.len())
    }
}
}

/// One column in a `return` block
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSpec {
    pub alias: Option<String>,
    pub var: String,
    pub order: Option<OrderDir>,
    pub distinct: bool,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl ColumnSpec {
    #[new]
    pub fn new(alias: Option<String>, var: String, order: Option<OrderDir>, distinct: bool, span: Option<Span>) -> Self {
        Self { alias, var, order, distinct, span }
    }
    fn __repr__(&self) -> String {
        format!("ColumnSpec({})", self.var)
    }
}
}

/// Sort direction for a return column
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderDir {
    Asc,
    Desc,
}

impl_python! {
#[pymethods]
impl OrderDir {
    fn __repr__(&self) -> &str {
        match self {
            OrderDir::Asc  => "OrderDir::Asc",
            OrderDir::Desc => "OrderDir::Desc",
        }
    }
}
}

/// A boolean comparison expression (used in bare filters and HAVING)
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct BoolExpr {
    pub left: BoolOperand,
    pub op: ComparisonOp,
    pub right: BoolOperand,
    pub span: Option<Span>,
}

impl_python! {
#[pymethods]
impl BoolExpr {
    #[new]
    pub fn new(left: BoolOperand, op: ComparisonOp, right: BoolOperand, span: Option<Span>) -> Self {
        Self { left, op, right, span }
    }
    fn __repr__(&self) -> String {
        format!("BoolExpr({:?} {:?})", self.left, self.right)
    }
}
}

/// One side of a boolean comparison
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum BoolOperand {
    Variable(String),
    Literal(Literal),
}

impl_python! {
#[pymethods]
impl BoolOperand {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            BoolOperand::Variable(_) => "variable",
            BoolOperand::Literal(_)  => "literal",
        }
    }
    fn __repr__(&self) -> String {
        match self {
            BoolOperand::Variable(v) => format!("BoolOperand::Variable({})", v),
            BoolOperand::Literal(_)  => "BoolOperand::Literal(...)".to_string(),
        }
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
    /// A query call pattern (invoke a named query with argument bindings)
    QueryCall {
        /// The name of the query being called
        name: QualifiedName,
        /// Argument bindings passed to the query
        args: Vec<QueryArg>,
        span: Option<Span>,
    },
    /// An inverse triple pattern (`subject is property of object`).
    /// Desugars to `object property subject`; `object` may be
    /// `Object::Constraint { block }` for the nested form.
    Inverse {
        /// The subject (the object of the underlying triple)
        subject: Subject,
        /// The property to match
        property: QualifiedName,
        /// The object (the subject of the underlying triple)
        object: Object,
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
            Pattern::QueryCall { .. } => "query_call",
            Pattern::Inverse { .. } => "inverse",
        }
    }

    #[getter]
    fn subject(&self) -> Option<Subject> {
        match self {
            Pattern::Triple { subject, .. }
            | Pattern::Type { subject, .. }
            | Pattern::Inverse { subject, .. } => Some(subject.clone()),
            Pattern::Quantified { .. } | Pattern::QueryCall { .. } => None,
        }
    }

    #[getter]
    fn property(&self) -> Option<QualifiedName> {
        match self {
            Pattern::Triple { property, .. } | Pattern::Inverse { property, .. } => {
                Some(property.clone())
            }
            _ => None,
        }
    }

    #[getter]
    fn object(&self) -> Option<Object> {
        match self {
            Pattern::Triple { object, .. } | Pattern::Inverse { object, .. } => {
                Some(object.clone())
            }
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

    #[getter]
    fn query_name(&self) -> Option<QualifiedName> {
        match self {
            Pattern::QueryCall { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn query_args(&self) -> Option<Vec<QueryArg>> {
        match self {
            Pattern::QueryCall { args, .. } => Some(args.clone()),
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
            Pattern::QueryCall { name, args, .. } => {
                format!("Pattern::QueryCall({} args={})", name, args.len())
            }
            Pattern::Inverse {
                subject,
                property,
                object,
                ..
            } => {
                format!("Pattern::Inverse({:?} is {} of {:?})", subject, property, object)
            }
        }
    }
}
}

/// Argument in a query call
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum QueryArg {
    /// Shorthand variable binding: ?var maps to same-named query param
    Var {
        name: String,
        span: Option<Span>,
    },
    /// Explicit binding: ?param = <value>
    Binding {
        param: String,
        value: Object,
        span: Option<Span>,
    },
}

impl_python! {
#[pymethods]
impl QueryArg {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            QueryArg::Var { .. } => "var",
            QueryArg::Binding { .. } => "binding",
        }
    }

    #[getter]
    fn name(&self) -> Option<String> {
        match self {
            QueryArg::Var { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn param(&self) -> Option<String> {
        match self {
            QueryArg::Binding { param, .. } => Some(param.clone()),
            _ => None,
        }
    }

    #[getter]
    fn value(&self) -> Option<Object> {
        match self {
            QueryArg::Binding { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            QueryArg::Var { name, .. } => format!("QueryArg::Var({})", name),
            QueryArg::Binding { param, .. } => format!("QueryArg::Binding({}=...)", param),
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
    Variable {
        name: String,
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
            Expr::Variable { .. } => "variable",
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
            Expr::Variable { name, .. } => format!("Expr::Variable({})", name),
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
            Expr::Variable { name, .. } => write!(f, "{}", name),
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
        binding: Option<String>,
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
    /// Inverse property usage: `is property of value`
    /// (`value` is the subject of the underlying triple, the constrained node the object).
    Inverse {
        property: QualifiedName,
        value: Object,
        span: Option<Span>,
    },
    /// Inverse property usage with a nested block: `is property of [ ... ]`
    InverseNested {
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
            Constraint::Inverse { .. } => "inverse",
            Constraint::InverseNested { .. } => "inverse_nested",
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
    fn binding(&self) -> Option<String> {
        match self {
            Constraint::Comparison { binding, .. } => binding.clone(),
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
            Constraint::Inverse { property, .. } => Some(property.clone()),
            Constraint::InverseNested { property, .. } => Some(property.clone()),
            _ => None,
        }
    }

    #[getter]
    fn block(&self) -> Option<ConstraintBlock> {
        match self {
            Constraint::PropertyConstraint { block, .. } => Some(block.clone()),
            Constraint::InverseNested { block, .. } => Some(block.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            Constraint::TypeIs { type_ref, .. } => format!("Constraint::TypeIs({:?})", type_ref),
            Constraint::Comparison { binding, operator, value, .. } => {
                format!("Constraint::Comparison({:?} {:?} {:?})", binding, operator, value)
            }
            Constraint::PropertyValue { property, value, .. } => {
                format!("Constraint::PropertyValue({} {:?})", property, value)
            }
            Constraint::PropertyConstraint { property, block, .. } => {
                format!("Constraint::PropertyConstraint({} {:?})", property, block)
            }
            Constraint::Inverse { property, value, .. } => {
                format!("Constraint::Inverse({} {:?})", property, value)
            }
            Constraint::InverseNested { property, block, .. } => {
                format!("Constraint::InverseNested({} {:?})", property, block)
            }
        }
    }
}
}
/// A file-header item: either a prefix statement or a temporal directive.
/// Used only transiently by the parser so prefixes and `@locale`/`@timezone`
/// directives may appear in any order at the top of a file.
pub enum HeaderItem {
    Prefixes(Vec<PrefixDecl>),
    Locale(String),
    Timezone(String),
}

/// The declared type of a temporal smart literal, i.e. which constructor
/// keyword introduced it (`date(...)`, `time(...)`, `date_time(...)`,
/// `duration(...)`). The value itself is parsed by the `dolfin-datetime` crate,
/// which also infers a type; the declared kind is validated against it.
#[cfg_attr(feature = "python", pyclass(eq, eq_int))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalKind {
    Date,
    Time,
    DateTime,
    Duration,
}

impl TemporalKind {
    /// The Dolfin constructor keyword.
    pub fn keyword(self) -> &'static str {
        match self {
            TemporalKind::Date => "date",
            TemporalKind::Time => "time",
            TemporalKind::DateTime => "date_time",
            TemporalKind::Duration => "duration",
        }
    }

    /// The XSD datatype IRI (compact form) this kind serialises to.
    pub fn xsd_type(self) -> &'static str {
        match self {
            TemporalKind::Date => "xsd:date",
            TemporalKind::Time => "xsd:time",
            TemporalKind::DateTime => "xsd:dateTime",
            TemporalKind::Duration => "xsd:duration",
        }
    }
}

impl fmt::Display for TemporalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.keyword())
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
    /// A temporal smart literal such as `date(June 1st 2026)`. `content` is the
    /// raw text between the parentheses, handed verbatim to `dolfin-datetime`.
    Temporal {
        kind: TemporalKind,
        content: String,
        span: Option<Span>,
    },
    /// A physical-quantity smart literal such as `quantity(42 km/h)`. `content`
    /// is the raw text between the parentheses, handed verbatim to
    /// `dolfin-units`. A trailing `as <unit>` inside `content` is a conversion
    /// directive resolved at parse time (see [`Literal::resolve_quantity`]).
    Quantity {
        content: String,
        span: Option<Span>,
    },
}

impl Literal {
    /// Resolve a temporal literal to its XSD lexical form and datatype, using
    /// the `dolfin-datetime` parser and the file-level `TemporalContext` built
    /// from `@locale` / `@timezone`. Returns `Ok((value, xsd_type))`, e.g.
    /// `("2026-06-01", "xsd:date")`. `None` for non-temporal literals.
    ///
    /// Pass `&TemporalContext::strict()` (or `&Default::default()`) when no
    /// file-level defaults apply; then numeric dates need an inline `as` mask
    /// and times are timezone-naive unless they carry an inline offset.
    pub fn resolve_temporal(
        &self,
        ctx: &dolfin_datetime::TemporalContext,
    ) -> Option<Result<(String, &'static str), String>> {
        let (kind, content) = match self {
            Literal::Temporal { kind, content, .. } => (*kind, content),
            _ => return None,
        };
        Some(
            dolfin_datetime::parse_temporal(content, ctx)
                .map_err(|e| e.to_string())
                .and_then(|expr| {
                    let got = temporal_kind_of(&expr);
                    if got == kind {
                        Ok((expr.to_xsd(), kind.xsd_type()))
                    } else {
                        Err(format!(
                            "{}(...) declared but value parses as a {}",
                            kind.keyword(),
                            got.keyword()
                        ))
                    }
                }),
        )
    }

    /// Resolve a `quantity(...)` literal via the `dolfin-units` parser, using
    /// only builtin units (no project-declared `unitdef`/`unitdef family`
    /// declarations — see [`Self::resolve_quantity_with`] for that). Returns
    /// the fully evaluated [`dolfin_units::Quantity`] (arithmetic and any
    /// trailing `as <unit>` conversion already applied), or a human-readable
    /// error string suitable for a Dolfin compile diagnostic. `None` for
    /// non-quantity literals.
    pub fn resolve_quantity(&self) -> Option<Result<dolfin_units::Quantity, String>> {
        self.resolve_quantity_with(&dolfin_units::UnitRegistry::with_defaults())
    }

    /// Same as [`Self::resolve_quantity`], but resolves unit tokens against
    /// `registry` first — pass a project-scoped registry (built from a
    /// package's `unit` declarations) so currency and nominal/family units
    /// the project declared itself are recognized.
    pub fn resolve_quantity_with(&self, registry: &dolfin_units::UnitRegistry) -> Option<Result<dolfin_units::Quantity, String>> {
        let content = match self {
            Literal::Quantity { content, .. } => content,
            _ => return None,
        };
        Some(dolfin_units::parse_quantity_with(content, registry).map_err(|e| e.to_string()))
    }
}

impl OntologyFile {
    /// Build a `dolfin-datetime` `TemporalContext` from this file's `@locale`
    /// and `@timezone` directives. Returns a context error string if a
    /// directive is malformed (bad locale order, unknown timezone). Absent
    /// directives yield a strict (default) context.
    pub fn temporal_context(&self) -> Result<dolfin_datetime::TemporalContext, String> {
        let locale = match &self.locale {
            Some(s) => Some(parse_locale_arg(s)?),
            None => None,
        };
        let timezone = match &self.timezone {
            Some(s) => {
                let tz = dolfin_datetime::Timezone::Named(s.clone());
                // Resolve eagerly so an unknown/unsupported zone surfaces here.
                tz.resolve().map_err(|e| e.to_string())?;
                Some(tz)
            }
            None => None,
        };
        Ok(dolfin_datetime::TemporalContext { locale, timezone })
    }
}

/// Parse a `@locale` argument like `d/m/y` into a `DateLocale`.
fn parse_locale_arg(arg: &str) -> Result<dolfin_datetime::DateLocale, String> {
    let sep = ['/', '-', '.']
        .into_iter()
        .find(|c| arg.contains(*c))
        .ok_or_else(|| format!("@locale '{arg}' needs a / - or . separator"))?;
    dolfin_datetime::parse_locale_order(arg, sep).map_err(|e| e.to_string())
}

fn temporal_kind_of(expr: &dolfin_datetime::TemporalExpr) -> TemporalKind {
    use dolfin_datetime::TemporalExpr as TE;
    match expr {
        TE::Date(_) => TemporalKind::Date,
        TE::Time(_) => TemporalKind::Time,
        TE::DateTime(_) => TemporalKind::DateTime,
        TE::Duration(_) => TemporalKind::Duration,
    }
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
            Literal::Temporal { .. } => "temporal",
            Literal::Quantity { .. } => "quantity",
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

    /// The raw content of a temporal literal (text between the parentheses).
    #[getter]
    fn temporal_value(&self) -> Option<String> {
        match self {
            Literal::Temporal { content, .. } => Some(content.clone()),
            _ => None,
        }
    }

    /// The raw content of a quantity literal (text between the parentheses).
    #[getter]
    fn quantity_value(&self) -> Option<String> {
        match self {
            Literal::Quantity { content, .. } => Some(content.clone()),
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
            Literal::Temporal { kind, content, .. } => {
                format!("Literal::Temporal({}, {:?})", kind.keyword(), content)
            }
            Literal::Quantity { content, .. } => {
                format!("Literal::Quantity({:?})", content)
            }
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
            Literal::Temporal { kind, content, .. } => write!(f, "{}({})", kind.keyword(), content),
            Literal::Quantity { content, .. } => write!(f, "quantity({})", content),
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

    /// Create a date type reference
    #[staticmethod]
    fn date() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Date,
            span: None,
        }
    }

    /// Create a date_time type reference
    #[staticmethod]
    fn date_time() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::DateTime,
            span: None,
        }
    }

    /// Create a time type reference
    #[staticmethod]
    fn time() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Time,
            span: None,
        }
    }

    /// Create a duration type reference
    #[staticmethod]
    fn duration() -> Self {
        TypeRef::Primitive {
            kind: PrimitiveKind::Duration,
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
    Date,
    DateTime,
    Time,
    Duration,
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
            PrimitiveKind::Date => "PrimitiveKind.Date",
            PrimitiveKind::DateTime => "PrimitiveKind.DateTime",
            PrimitiveKind::Time => "PrimitiveKind.Time",
            PrimitiveKind::Duration => "PrimitiveKind.Duration",
        }
    }

    fn __str__(&self) -> &str {
        match self {
            PrimitiveKind::String => "string",
            PrimitiveKind::Int => "int",
            PrimitiveKind::Float => "float",
            PrimitiveKind::Boolean => "boolean",
            PrimitiveKind::Date => "date",
            PrimitiveKind::DateTime => "date_time",
            PrimitiveKind::Time => "time",
            PrimitiveKind::Duration => "duration",
        }
    }
}
}

impl fmt::Display for PrimitiveKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.__str__())
    }
}

// ============================================================================
// FACT (INSTANCE) AST NODES
// ============================================================================

/// A value in a fact assertion
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum FactValue {
    /// Scalar literal (string, int, float, boolean, IRI)
    Literal { value: Literal, span: Option<Span> },
    /// Instance reference: `:name` or `prefix:name`
    Reference { qualifier: Option<String>, name: String, span: Option<Span> },
    /// Enum member or named concept value (dot-separated name)
    Named { name: QualifiedName, span: Option<Span> },
    /// Inline anonymous block `[ ... ]` (blank node)
    Block { type_hint: Option<QualifiedName>, assertions: Vec<FactAssertion>, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl FactValue {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            FactValue::Literal { .. } => "literal",
            FactValue::Reference { .. } => "reference",
            FactValue::Named { .. } => "named",
            FactValue::Block { .. } => "block",
        }
    }

    #[getter]
    fn literal(&self) -> Option<Literal> {
        match self {
            FactValue::Literal { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn qualifier(&self) -> Option<String> {
        match self {
            FactValue::Reference { qualifier, .. } => qualifier.clone(),
            _ => None,
        }
    }

    #[getter]
    fn name(&self) -> Option<String> {
        match self {
            FactValue::Reference { name, .. } => Some(name.clone()),
            FactValue::Named { name, .. } => Some(name.full()),
            _ => None,
        }
    }

    #[getter]
    fn qualified_name(&self) -> Option<QualifiedName> {
        match self {
            FactValue::Named { name, .. } => Some(name.clone()),
            _ => None,
        }
    }

    #[getter]
    fn type_hint(&self) -> Option<QualifiedName> {
        match self {
            FactValue::Block { type_hint, .. } => type_hint.clone(),
            _ => None,
        }
    }

    #[getter]
    fn assertions(&self) -> Option<Vec<FactAssertion>> {
        match self {
            FactValue::Block { assertions, .. } => Some(assertions.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            FactValue::Literal { value, .. } => format!("FactValue::Literal({:?})", value),
            FactValue::Reference { qualifier, name, .. } => match qualifier {
                Some(q) => format!("FactValue::Reference({}:{})", q, name),
                None => format!("FactValue::Reference(:{})  ", name),
            },
            FactValue::Named { name, .. } => format!("FactValue::Named({})", name.full()),
            FactValue::Block { .. } => "FactValue::Block(...)".to_string(),
        }
    }
}
}

/// A single assertion in a fact body
#[cfg_attr(feature = "python", pyclass(frozen, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub enum FactAssertion {
    /// `property_name value, ...`
    Property { property: QualifiedName, values: Vec<FactValue>, span: Option<Span> },
    /// `is property of value`  (inverse property form)
    Inverse { property: QualifiedName, value: FactValue, span: Option<Span> },
    /// `a ConceptName`  (type hint inside anonymous block)
    TypeHint { type_ref: QualifiedName, span: Option<Span> },
}

impl_python! {
#[pymethods]
impl FactAssertion {
    #[getter]
    fn kind(&self) -> &str {
        match self {
            FactAssertion::Property { .. } => "property",
            FactAssertion::Inverse { .. } => "inverse",
            FactAssertion::TypeHint { .. } => "type_hint",
        }
    }

    #[getter]
    fn values(&self) -> Option<Vec<FactValue>> {
        match self {
            FactAssertion::Property { values, .. } => Some(values.clone()),
            _ => None,
        }
    }

    #[getter]
    fn property(&self) -> Option<QualifiedName> {
        match self {
            FactAssertion::Inverse { property, .. } => Some(property.clone()),
            FactAssertion::Property { property, .. } => Some(property.clone()),
            _ => None,
        }
    }

    #[getter]
    fn value(&self) -> Option<FactValue> {
        match self {
            FactAssertion::Inverse { value, .. } => Some(value.clone()),
            _ => None,
        }
    }

    #[getter]
    fn type_ref(&self) -> Option<QualifiedName> {
        match self {
            FactAssertion::TypeHint { type_ref, .. } => Some(type_ref.clone()),
            _ => None,
        }
    }

    fn __repr__(&self) -> String {
        match self {
            FactAssertion::Property { property, values, .. } => {
                format!("FactAssertion::Property({}, {} value(s))", property, values.len())
            }
            FactAssertion::Inverse { property, .. } => {
                format!("FactAssertion::Inverse({})", property.full())
            }
            FactAssertion::TypeHint { type_ref, .. } => {
                format!("FactAssertion::TypeHint({})", type_ref.full())
            }
        }
    }
}
}

/// A fact (instance) definition
#[cfg_attr(feature = "python", pyclass(frozen, get_all, from_py_object))]
#[derive(Debug, Clone, PartialEq)]
pub struct FactDef {
    /// The instance identifier
    pub id: String,
    /// The concept(s) this instance is a member of
    pub types: Vec<QualifiedName>,
    /// Property assertions and inverse assertions
    pub assertions: Vec<FactAssertion>,
    pub span: Option<Span>,
    /// Span of just the id token (for rename support)
    pub id_span: Option<Span>,
}

impl_python! {
#[pymethods]
impl FactDef {
    #[new]
    #[pyo3(signature = (id, types, assertions, span=None))]
    pub fn new(id: String, types: Vec<QualifiedName>, assertions: Vec<FactAssertion>, span: Option<Span>) -> Self {
        Self { id, types, assertions, span, id_span: None }
    }

    fn __repr__(&self) -> String {
        format!("FactDef('{}', types={}, assertions={})", self.id, self.types.len(), self.assertions.len())
    }
}
}
