use std::collections::HashMap;
use std::convert::TryFrom;
use std::fmt::{self, Debug, Display, Formatter};
use std::ops::{Add, AddAssign, Deref};
use std::rc::Rc;

use super::{Str, Value};
use crate::diag::StrResult;
use crate::exec::ExecContext;
use crate::syntax::{Expr, SyntaxTree};
use crate::util::EcoString;

/// A template value: `[*Hi* there]`.
#[derive(Debug, Default, Clone)]
pub struct Template {
    nodes: Rc<Vec<TemplateNode>>,
}

impl Template {
    /// Create a new template from a vector of nodes.
    pub fn new(nodes: Vec<TemplateNode>) -> Self {
        Self { nodes: Rc::new(nodes) }
    }

    /// Iterate over the contained template nodes.
    pub fn iter(&self) -> std::slice::Iter<TemplateNode> {
        self.nodes.iter()
    }

    /// Repeat this template `n` times.
    pub fn repeat(&self, n: i64) -> StrResult<Self> {
        let count = usize::try_from(n)
            .ok()
            .and_then(|n| self.nodes.len().checked_mul(n))
            .ok_or_else(|| format!("cannot repeat this template {} times", n))?;

        Ok(Self {
            nodes: Rc::new(self.iter().cloned().cycle().take(count).collect()),
        })
    }
}

impl Display for Template {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.pad("<template>")
    }
}

impl PartialEq for Template {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.nodes, &other.nodes)
    }
}

impl Add for Template {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for Template {
    fn add_assign(&mut self, rhs: Template) {
        let sink = Rc::make_mut(&mut self.nodes);
        match Rc::try_unwrap(rhs.nodes) {
            Ok(source) => sink.extend(source),
            Err(rc) => sink.extend(rc.iter().cloned()),
        }
    }
}

impl Add<Str> for Template {
    type Output = Self;

    fn add(mut self, rhs: Str) -> Self::Output {
        Rc::make_mut(&mut self.nodes).push(TemplateNode::Str(rhs.into()));
        self
    }
}

impl Add<Template> for Str {
    type Output = Template;

    fn add(self, mut rhs: Template) -> Self::Output {
        Rc::make_mut(&mut rhs.nodes).insert(0, TemplateNode::Str(self.into()));
        rhs
    }
}

impl From<TemplateTree> for Template {
    fn from(tree: TemplateTree) -> Self {
        Self::new(vec![TemplateNode::Tree(tree)])
    }
}

impl From<TemplateFunc> for Template {
    fn from(func: TemplateFunc) -> Self {
        Self::new(vec![TemplateNode::Func(func)])
    }
}

impl From<Str> for Template {
    fn from(string: Str) -> Self {
        Self::new(vec![TemplateNode::Str(string.into())])
    }
}

/// One node of a template.
///
/// Evaluating a template expression creates only a single node. Adding multiple
/// templates can yield multi-node templates.
#[derive(Debug, Clone)]
pub enum TemplateNode {
    /// A template that was evaluated from a template expression.
    Tree(TemplateTree),
    /// A function template that can implement custom behaviour.
    Func(TemplateFunc),
    /// A template that was converted from a string.
    Str(EcoString),
}

/// A template that consists of a syntax tree plus already evaluated
/// expressions.
#[derive(Debug, Clone)]
pub struct TemplateTree {
    /// The syntax tree of the corresponding template expression.
    pub tree: Rc<SyntaxTree>,
    /// The evaluated expressions in the syntax tree.
    pub map: ExprMap,
}

/// A map from expressions to the values they evaluated to.
///
/// The raw pointers point into the expressions contained in some
/// [`SyntaxTree`]. Since the lifetime is erased, the tree could go out of scope
/// while the hash map still lives. Although this could lead to lookup panics,
/// it is safe since the pointers are never dereferenced.
pub type ExprMap = HashMap<*const Expr, Value>;

/// A reference-counted dynamic template node that can implement custom
/// behaviour.
#[derive(Clone)]
pub struct TemplateFunc(Rc<dyn Fn(&mut ExecContext)>);

impl TemplateFunc {
    /// Create a new function template from a rust function or closure.
    pub fn new<F>(f: F) -> Self
    where
        F: Fn(&mut ExecContext) + 'static,
    {
        Self(Rc::new(f))
    }
}

impl Deref for TemplateFunc {
    type Target = dyn Fn(&mut ExecContext);

    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}

impl Debug for TemplateFunc {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("TemplateFunc").finish()
    }
}
