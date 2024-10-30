#[doc(inline)]
pub use typst_macros::category;

use std::fmt::{self, Debug, Formatter};
use std::hash::{Hash, Hasher};

use ecow::{eco_format, EcoString};
use indexmap::IndexMap;
use typst_syntax::ast::{self, AstNode};
use typst_syntax::Span;
use typst_utils::Static;

use crate::diag::{bail, HintedStrResult, HintedString, StrResult};
use crate::foundations::{
    Element, Func, IntoValue, Module, NativeElement, NativeFunc, NativeFuncData,
    NativeType, Type, Value,
};
use crate::Library;

/// A stack of scopes.
#[derive(Debug, Default, Clone)]
pub struct Scopes<'a> {
    /// The active scope.
    pub top: Scope,
    /// The stack of lower scopes.
    pub scopes: Vec<Scope>,
    /// The standard library.
    pub base: Option<&'a Library>,
}

impl<'a> Scopes<'a> {
    /// Create a new, empty hierarchy of scopes.
    pub fn new(base: Option<&'a Library>) -> Self {
        Self { top: Scope::new(), scopes: vec![], base }
    }

    /// Enter a new scope.
    pub fn enter(&mut self) {
        self.scopes.push(std::mem::take(&mut self.top));
    }

    /// Exit the topmost scope.
    ///
    /// This panics if no scope was entered.
    pub fn exit(&mut self) {
        self.top = self.scopes.pop().expect("no pushed scope");
    }

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> HintedStrResult<&Value> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| scope.get(var))
            .or_else(|| {
                self.base.and_then(|base| match base.global.scope().get(var) {
                    Some(value) => Some(value),
                    None if var == "std" => Some(&base.std),
                    None => None,
                })
            })
            .ok_or_else(|| unknown_variable(var))
    }

    /// Try to access a variable immutably in math.
    pub fn get_in_math(&self, var: &str) -> HintedStrResult<&Value> {
        std::iter::once(&self.top)
            .chain(self.scopes.iter().rev())
            .find_map(|scope| scope.get(var))
            .or_else(|| {
                self.base.and_then(|base| match base.math.scope().get(var) {
                    Some(value) => Some(value),
                    None if var == "std" => Some(&base.std),
                    None => None,
                })
            })
            .ok_or_else(|| {
                unknown_variable_math(
                    var,
                    self.base.is_some_and(|base| base.global.scope().get(var).is_some()),
                )
            })
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> HintedStrResult<&mut Value> {
        std::iter::once(&mut self.top)
            .chain(&mut self.scopes.iter_mut().rev())
            .find_map(|scope| scope.get_mut(var))
            .ok_or_else(|| {
                match self.base.and_then(|base| base.global.scope().get(var)) {
                    Some(_) => cannot_mutate_constant(var),
                    _ if var == "std" => cannot_mutate_constant(var),
                    _ => unknown_variable(var),
                }
            })?
    }

    /// Check if an std variable is shadowed.
    pub fn check_std_shadowed(&self, var: &str) -> bool {
        self.base.is_some_and(|base| base.global.scope().get(var).is_some())
            && std::iter::once(&self.top)
                .chain(self.scopes.iter().rev())
                .any(|scope| scope.get(var).is_some())
    }
}

#[cold]
fn cannot_mutate_constant(var: &str) -> HintedString {
    eco_format!("cannot mutate a constant: {}", var).into()
}

/// The error message when a variable is not found.
#[cold]
fn unknown_variable(var: &str) -> HintedString {
    let mut res = HintedString::new(eco_format!("unknown variable: {}", var));

    if var.contains('-') {
        res.hint(eco_format!(
            "if you meant to use subtraction, try adding spaces around the minus sign{}: `{}`",
            if var.matches('-').count() > 1 { "s" } else { "" },
            var.replace('-', " - ")
        ));
    }

    res
}

#[cold]
fn unknown_variable_math(var: &str, in_global: bool) -> HintedString {
    let mut res = HintedString::new(eco_format!("unknown variable: {}", var));

    if matches!(var, "none" | "auto" | "false" | "true") {
        res.hint(eco_format!(
            "if you meant to use a literal, try adding a hash before it: `#{var}`",
        ));
    } else if in_global {
        res.hint(eco_format!(
            "`{var}` is not available directly in math, try adding a hash before it: `#{var}`",
        ));
    } else {
        res.hint(eco_format!(
            "if you meant to display multiple letters as is, try adding spaces between each letter: `{}`",
            var.chars()
                .flat_map(|c| [' ', c])
                .skip(1)
                .collect::<EcoString>()
        ));
        res.hint(eco_format!(
            "or if you meant to display this as text, try placing it in quotes: `\"{var}\"`"
        ));
    }

    res
}

/// A map from binding names to values.
#[derive(Default, Clone)]
pub struct Scope {
    map: IndexMap<EcoString, Slot>,
    deduplicate: bool,
    category: Option<Category>,
}

impl Scope {
    /// Create a new empty scope.
    pub fn new() -> Self {
        Default::default()
    }

    /// Create a new scope with duplication prevention.
    pub fn deduplicating() -> Self {
        Self { deduplicate: true, ..Default::default() }
    }

    /// Enter a new category.
    pub fn category(&mut self, category: Category) {
        self.category = Some(category);
    }

    /// Reset the category.
    pub fn reset_category(&mut self) {
        self.category = None;
    }

    /// Bind a value to a name.
    #[track_caller]
    pub fn define(&mut self, name: impl Into<EcoString>, value: impl IntoValue) {
        self.define_spanned(name, value, Span::detached())
    }

    /// Bind a value to a name defined by an identifier.
    #[track_caller]
    pub fn define_ident(&mut self, ident: ast::Ident, value: impl IntoValue) {
        self.define_spanned(ident.get().clone(), value, ident.span())
    }

    /// Bind a value to a name.
    #[track_caller]
    pub fn define_spanned(
        &mut self,
        name: impl Into<EcoString>,
        value: impl IntoValue,
        span: Span,
    ) {
        let name = name.into();

        #[cfg(debug_assertions)]
        if self.deduplicate && self.map.contains_key(&name) {
            panic!("duplicate definition: {name}");
        }

        self.map.insert(
            name,
            Slot::new(value.into_value(), span, Kind::Normal, self.category),
        );
    }

    /// Define a captured, immutable binding.
    pub fn define_captured(
        &mut self,
        name: EcoString,
        value: Value,
        capturer: Capturer,
        span: Span,
    ) {
        self.map.insert(
            name,
            Slot::new(value.into_value(), span, Kind::Captured(capturer), self.category),
        );
    }

    /// Define a native function through a Rust type that shadows the function.
    pub fn define_func<T: NativeFunc>(&mut self) {
        let data = T::data();
        self.define(data.name, Func::from(data));
    }

    /// Define a native function with raw function data.
    pub fn define_func_with_data(&mut self, data: &'static NativeFuncData) {
        self.define(data.name, Func::from(data));
    }

    /// Define a native type.
    pub fn define_type<T: NativeType>(&mut self) {
        let data = T::data();
        self.define(data.name, Type::from(data));
    }

    /// Define a native element.
    pub fn define_elem<T: NativeElement>(&mut self) {
        let data = T::data();
        self.define(data.name, Element::from(data));
    }

    /// Define a module.
    pub fn define_module(&mut self, module: Module) {
        self.define(module.name().clone(), module);
    }

    /// Try to access a variable immutably.
    pub fn get(&self, var: &str) -> Option<&Value> {
        self.map.get(var).map(Slot::read)
    }

    /// Try to access a variable mutably.
    pub fn get_mut(&mut self, var: &str) -> Option<HintedStrResult<&mut Value>> {
        self.map
            .get_mut(var)
            .map(Slot::write)
            .map(|res| res.map_err(HintedString::from))
    }

    /// Get the span of a definition.
    pub fn get_span(&self, var: &str) -> Option<Span> {
        Some(self.map.get(var)?.span)
    }

    /// Get the category of a definition.
    pub fn get_category(&self, var: &str) -> Option<Category> {
        self.map.get(var)?.category
    }

    /// Iterate over all definitions.
    pub fn iter(&self) -> impl Iterator<Item = (&EcoString, &Value, Span)> {
        self.map.iter().map(|(k, v)| (k, v.read(), v.span))
    }
}

impl Debug for Scope {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Scope ")?;
        f.debug_map()
            .entries(self.map.iter().map(|(k, v)| (k, v.read())))
            .finish()
    }
}

impl Hash for Scope {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.map.len());
        for item in &self.map {
            item.hash(state);
        }
        self.deduplicate.hash(state);
        self.category.hash(state);
    }
}

/// Defines the associated scope of a Rust type.
pub trait NativeScope {
    /// The constructor function for the type, if any.
    fn constructor() -> Option<&'static NativeFuncData>;

    /// Get the associated scope for the type.
    fn scope() -> Scope;
}

/// A slot where a value is stored.
#[derive(Clone, Hash)]
struct Slot {
    /// The stored value.
    value: Value,
    /// The kind of slot, determines how the value can be accessed.
    kind: Kind,
    /// A span associated with the stored value.
    span: Span,
    /// The category of the slot.
    category: Option<Category>,
}

/// The different kinds of slots.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
enum Kind {
    /// A normal, mutable binding.
    Normal,
    /// A captured copy of another variable.
    Captured(Capturer),
}

/// What the variable was captured by.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum Capturer {
    /// Captured by a function / closure.
    Function,
    /// Captured by a context expression.
    Context,
}

impl Slot {
    /// Create a new slot.
    fn new(value: Value, span: Span, kind: Kind, category: Option<Category>) -> Self {
        Self { value, span, kind, category }
    }

    /// Read the value.
    fn read(&self) -> &Value {
        &self.value
    }

    /// Try to write to the value.
    fn write(&mut self) -> StrResult<&mut Value> {
        match self.kind {
            Kind::Normal => Ok(&mut self.value),
            Kind::Captured(capturer) => {
                bail!(
                    "variables from outside the {} are \
                     read-only and cannot be modified",
                    match capturer {
                        Capturer::Function => "function",
                        Capturer::Context => "context expression",
                    }
                )
            }
        }
    }
}

/// A group of related definitions.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Category(Static<CategoryData>);

impl Category {
    /// Create a new category from raw data.
    pub const fn from_data(data: &'static CategoryData) -> Self {
        Self(Static(data))
    }

    /// The category's name.
    pub fn name(&self) -> &'static str {
        self.0.name
    }

    /// The type's title case name, for use in documentation (e.g. `String`).
    pub fn title(&self) -> &'static str {
        self.0.title
    }

    /// Documentation for the category.
    pub fn docs(&self) -> &'static str {
        self.0.docs
    }
}

impl Debug for Category {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Category({})", self.name())
    }
}

/// Defines a category.
#[derive(Debug)]
pub struct CategoryData {
    pub name: &'static str,
    pub title: &'static str,
    pub docs: &'static str,
}
