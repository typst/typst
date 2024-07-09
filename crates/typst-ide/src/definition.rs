use ecow::EcoString;
use typst::foundations::{Label, Module, Selector, Value};
use typst::model::Document;
use typst::syntax::ast::AstNode;
use typst::syntax::{ast, LinkedNode, Side, Source, Span, SyntaxKind};
use typst::World;

use crate::{analyze_import, deref_target, named_items, DerefTarget, NamedItem};

/// Find the definition of the item under the cursor.
///
/// Passing a `document` (from a previous compilation) is optional, but enhances
/// the definition search. Label definitions, for instance, are only generated
/// when the document is available.
pub fn definition(
    world: &dyn World,
    document: Option<&Document>,
    source: &Source,
    cursor: usize,
    side: Side,
) -> Option<Definition> {
    let root = LinkedNode::new(source.root());
    let leaf = root.leaf_at(cursor, side)?;

    let target = deref_target(leaf.clone())?;

    let mut use_site = match target {
        DerefTarget::VarAccess(node) | DerefTarget::Callee(node) => node,
        DerefTarget::IncludePath(path) | DerefTarget::ImportPath(path) => {
            let import_item =
                analyze_import(world, &path).and_then(|v| v.cast::<Module>().ok())?;
            return Some(Definition::module(&import_item, path.span(), Span::detached()));
        }
        DerefTarget::Ref(r) => {
            let ref_node = r.cast::<ast::Ref>()?.target();
            let sel = Selector::Label(Label::new(ref_node));
            let elem = document?.introspector.query_first(&sel)?;
            let span = elem.span();

            return Some(Definition {
                kind: DefinitionKind::Label,
                name: r.text().clone(),
                value: elem.label().map(Value::Label),
                span,
                name_span: Span::detached(),
            });
        }
        DerefTarget::Label(..) | DerefTarget::Code(..) => {
            return None;
        }
    };

    let mut has_path = false;
    while let Some(node) = use_site.cast::<ast::FieldAccess>() {
        has_path = true;
        use_site = use_site.find(node.target().span())?;
    }

    let name = use_site.cast::<ast::Ident>()?.get().clone();
    let src = named_items(world, use_site, |item: NamedItem| {
        if *item.name() != name {
            return None;
        }

        match item {
            NamedItem::Var(name) => {
                let name_span = name.span();
                let span = find_let_binding(source, name_span);
                Some(Definition::item(name.get().clone(), span, name_span, None))
            }
            NamedItem::Fn(name) => {
                let name_span = name.span();
                let span = find_let_binding(source, name_span);
                Some(
                    Definition::item(name.get().clone(), span, name_span, None)
                        .with_kind(DefinitionKind::Function),
                )
            }
            NamedItem::Module(item, site) => Some(Definition::module(
                item,
                site.span(),
                matches!(site.kind(), SyntaxKind::Ident)
                    .then_some(site.span())
                    .unwrap_or_else(Span::detached),
            )),
            NamedItem::Import(name, span, value) => Some(Definition::item(
                name.clone(),
                Span::detached(),
                span,
                value.cloned(),
            )),
        }
    });

    let src = src.or_else(|| {
        let in_math = matches!(
            leaf.parent_kind(),
            Some(SyntaxKind::Equation)
                | Some(SyntaxKind::Math)
                | Some(SyntaxKind::MathFrac)
                | Some(SyntaxKind::MathAttach)
        );
        let library = world.library();

        let scope = if in_math { library.math.scope() } else { library.global.scope() };
        for (item_name, value) in scope.iter() {
            if *item_name == name {
                return Some(Definition::item(
                    name,
                    Span::detached(),
                    Span::detached(),
                    Some(value.clone()),
                ));
            }
        }

        None
    })?;

    (!has_path).then_some(src)
}

/// A definition of some item.
#[derive(Debug, Clone)]
pub struct Definition {
    /// The name of the definition.
    pub name: EcoString,
    /// The kind of the definition.
    pub kind: DefinitionKind,
    /// An instance of the definition, if available.
    pub value: Option<Value>,
    /// The source span of the entire definition. May be detached if unknown.
    pub span: Span,
    /// The span of the definition's name. May be detached if unknown.
    pub name_span: Span,
}

impl Definition {
    fn item(name: EcoString, span: Span, name_span: Span, value: Option<Value>) -> Self {
        Self {
            name,
            kind: match value {
                Some(Value::Func(_)) => DefinitionKind::Function,
                _ => DefinitionKind::Variable,
            },
            value,
            span,
            name_span,
        }
    }

    fn module(module: &Module, span: Span, name_span: Span) -> Self {
        Definition {
            name: module.name().clone(),
            kind: DefinitionKind::Module(module.clone()),
            value: Some(Value::Module(module.clone())),
            span,
            name_span,
        }
    }

    fn with_kind(self, kind: DefinitionKind) -> Self {
        Self { kind, ..self }
    }
}

/// A kind of item that is definition.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum DefinitionKind {
    /// ```plain
    /// let foo;
    /// ^^^^^^^^ span
    ///     ^^^ name_span
    /// ```
    Variable,
    /// ```plain
    /// let foo(it) = it;
    /// ^^^^^^^^^^^^^^^^^ span
    ///     ^^^ name_span
    /// ```
    Function,
    /// Case 1
    /// ```plain
    /// import "foo.typ": *
    ///        ^^^^^^^^^ span
    /// name_span is detached
    /// ```
    ///
    /// Case 2
    /// ```plain
    /// import "foo.typ" as bar: *
    ///                span ^^^
    ///           name_span ^^^
    /// ```
    ///
    /// Some modules are not associated with a file, like the built-in modules.
    Module(Module),
    /// ```plain
    /// <foo>
    /// ^^^^^ span
    /// name_span is detached
    /// ```
    Label,
}

fn find_let_binding(source: &Source, name_span: Span) -> Span {
    let node = LinkedNode::new(source.root());
    std::iter::successors(node.find(name_span).as_ref(), |n| n.parent())
        .find(|n| matches!(n.kind(), SyntaxKind::LetBinding))
        .map(|s| s.span())
        .unwrap_or_else(Span::detached)
}

#[cfg(test)]
mod tests {
    use typst::foundations::Value;
    use typst::syntax::{Side, Span};

    use super::{definition, Definition};
    use crate::tests::TestWorld;

    fn var(text: &str, value: bool) -> Option<Definition> {
        Some(Definition::item(
            text.into(),
            Span::detached(),
            Span::detached(),
            if value { Some(Value::Bool(false)) } else { None },
        ))
    }

    fn func(text: &str, value: bool) -> Option<Definition> {
        var(text, value).map(|d| d.with_kind(super::DefinitionKind::Function))
    }

    #[track_caller]
    fn test(text: &str, cursor: usize, expected: Option<Definition>) {
        let world = TestWorld::new(text);
        let doc = typst::compile(&world).output.ok();
        let actual = definition(&world, doc.as_ref(), &world.main, cursor, Side::After);
        let actual = actual.map(|d| (d.kind, d.name, d.value.is_some()));
        let expected = expected.map(|d| (d.kind, d.name, d.value.is_some()));
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_definition() {
        test("#let x; #x", 9, var("x", false));
        test("#let x() = {}; #x", 16, func("x", false));
        test("#table", 1, func("table", true));
    }
}
