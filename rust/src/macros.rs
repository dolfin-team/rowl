// macros.rs

//! Macros for conditional PyO3 bindings.
//!
//! These macros allow defining Python bindings that are only compiled
//! when the `python` feature is enabled. When disabled, the code compiles
//! as plain Rust without any PyO3 dependency.
//!
//! # Usage
//!
//! ```ignore
//! use rowl::macros::impl_python;
//!
//! impl_python! {
//!     #[pymethods]
//!     impl MyStruct {
//!         #[new]
//!         pub fn new(name: String, value: i64) -> Self {
//!             Self { name, value }
//!         }
//!
//!         #[getter]
//!         pub fn display_name(&self) -> String {
//!             format!("{}: {}", self.name, self.value)
//!         }
//!
//!         fn __repr__(&self) -> String {
//!             format!("MyStruct('{}', {})", self.name, self.value)
//!         }
//!     }
//! }
//! ```

// ============================================================================
// impl_python! — Conditional PyO3 impl blocks
// ============================================================================

/// Define an impl block with conditional PyO3 method attributes.
///
/// When `feature = "python"` is enabled, the first `#[...]` attribute
/// (e.g. `#[pymethods]`) is applied, and inner method attributes like
/// `#[new]`, `#[getter]`, `#[staticmethod]` are preserved.
///
/// When disabled, the outer attribute is stripped and inner PyO3-specific
/// attributes are silently removed, yielding plain Rust methods.
///
/// # Example
///
/// ```rust,ignore
/// impl_python! {
///     #[pymethods]
///     impl QualifiedName {
///         #[new]
///         pub fn new(parts: Vec<String>) -> Self {
///             Self { parts }
///         }
///
///         #[staticmethod]
///         pub fn from_single(name: String) -> Self {
///             Self { parts: vec![name] }
///         }
///
///         #[getter]
///         pub fn full(&self) -> String {
///             self.parts.join(".")
///         }
///
///         fn __repr__(&self) -> String {
///             format!("QualifiedName('{}')", self.full())
///         }
///     }
/// }
/// ```
///
/// # Supported signatures
///
/// - Associated functions: `fn name(arg: Type) -> Ret { ... }`
/// - Methods with `&self`: `fn name(&self, arg: Type) -> Ret { ... }`
/// - Methods with `&mut self`: `fn name(&mut self, arg: Type) -> Ret { ... }`
/// - PyO3 signature attribute: `#[pyo3(signature = (...))]`
#[cfg(feature = "python")]
macro_rules! impl_python {
    (
        #[$py_attr:meta]
        impl $type:ty {
            $(
                $(#[$meta:meta])*
                $vis:vis fn $name:ident( $($params:tt)* ) $(-> $ret:ty)? $body:block
            )*
        }
    ) => {
        #[$py_attr]
        impl $type {
            $(
                $(#[$meta])*
                $vis fn $name( $($params)* ) $(-> $ret)? $body
            )*
        }
    };
}

#[cfg(not(feature = "python"))]
macro_rules! impl_python {
    (
        #[$py_attr:meta]
        impl $type:ty {
            $(
                $(#[$meta:meta])*
                $vis:vis fn $name:ident( $($params:tt)* ) $(-> $ret:ty)? $body:block
            )*
        }
    ) => {
        impl $type {
            $(
                $vis fn $name( $($params)* ) $(-> $ret)? $body
            )*
        }
    };
}

// ============================================================================
// py_only! — Conditionally includes content only when Python feature is active
// ============================================================================

#[cfg(feature = "python")]
macro_rules! py_only {
    ( $($content:tt)* ) => {
        $($content)*
    };
}

#[cfg(not(feature = "python"))]
macro_rules! py_only {
    ( $($content:tt)* ) => {};
}

pub(crate) use py_only;

// ============================================================================
// Re-exports
// ============================================================================

pub(crate) use impl_python;
