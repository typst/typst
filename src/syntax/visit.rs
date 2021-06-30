//! Mutable and immutable syntax tree traversal.

use crate::syntax::*;

/// Implement the immutable and the mutable visitor version.
macro_rules! impl_visitors {
    ($($name:ident($($tts:tt)*) $body:block)*) => {
        macro_rules! r {
            (rc: $x:expr) => { $x.as_ref() };
            ($x:expr) => { &$x };
        }

        impl_visitor! {
            /// Walk syntax trees immutably.
            Visit,
            /// Immutable visitor functions.
            immutable,
            [$(($name($($tts)*) $body))*]
        }

        macro_rules! r {
            (rc: $x:expr) => { std::rc::Rc::make_mut(&mut $x) };
            ($x:expr) => { &mut $x };
        }

        impl_visitor! {
            /// Walk syntax trees mutably.
            VisitMut,
            /// Mutable visitor functions.
            mutable,
            [$(($name($($tts)*) $body mut))*] mut
        }
    };
}

/// Implement an immutable or mutable visitor.
macro_rules! impl_visitor {
    (
        #[doc = $visit_doc:expr] $visit:ident,
        #[doc = $module_doc:expr] $module:ident,
        [$((
            $name:ident($v:ident, $node:ident: $ty:ty)
            $body:block
            $($fmut:tt)?
        ))*]
        $($mut:tt)?
    ) => {
        #[doc = $visit_doc]
        pub trait $visit<'ast> {
            /// Visit a definition of a binding.
            ///
            /// Bindings are, for example, left-hand side of let expressions,
            /// and key/value patterns in for loops.
            fn visit_binding(&mut self, _: &'ast $($mut)? Ident) {}

            /// Visit the entry into a scope.
            fn visit_enter(&mut self) {}

            /// Visit the exit from a scope.
            fn visit_exit(&mut self) {}

            $(fn $name(&mut self, $node: &'ast $($fmut)? $ty) {
                $module::$name(self, $node);
            })*
        }

        #[doc = $module_doc]
        pub mod $module {
            use super::*;
            $(
                #[allow(unused_variables)]
                pub fn $name<'ast, V>($v: &mut V, $node: &'ast $($fmut)? $ty)
                where
                    V: $visit<'ast> + ?Sized
                $body
            )*
        }
    };
}

impl_visitors! {
    visit_tree(v, tree: SyntaxTree) {
        for node in tree {
            v.visit_node(node);
        }
    }

    visit_node(v, node: Node) {
        match node {
            Node::Text(_) => {}
            Node::Space => {}
            Node::Linebreak(_) => {}
            Node::Parbreak(_) => {}
            Node::Strong(_) => {}
            Node::Emph(_) => {}
            Node::Raw(_) => {}
            Node::Heading(n) => v.visit_heading(n),
            Node::List(n) => v.visit_list(n),
            Node::Enum(n) => v.visit_enum(n),
            Node::Expr(n) => v.visit_expr(n),
        }
    }

    visit_heading(v, heading: HeadingNode) {
        v.visit_tree(r!(rc: heading.body));
    }

    visit_list(v, item: ListItem) {
        v.visit_tree(r!(item.body));
    }

    visit_enum(v, item: EnumItem) {
        v.visit_tree(r!(item.body));
    }

    visit_expr(v, expr: Expr) {
        match expr {
            Expr::None(_) => {}
            Expr::Auto(_) => {}
            Expr::Bool(_, _) => {}
            Expr::Int(_, _) => {}
            Expr::Float(_, _) => {}
            Expr::Length(_, _, _) => {}
            Expr::Angle(_, _, _) => {}
            Expr::Percent(_, _) => {}
            Expr::Fractional(_, _) => {}
            Expr::Color(_, _) => {}
            Expr::Str(_, _) => {}
            Expr::Ident(_) => {}
            Expr::Array(e) => v.visit_array(e),
            Expr::Dict(e) => v.visit_dict(e),
            Expr::Template(e) => v.visit_template(e),
            Expr::Group(e) => v.visit_group(e),
            Expr::Block(e) => v.visit_block(e),
            Expr::Unary(e) => v.visit_unary(e),
            Expr::Binary(e) => v.visit_binary(e),
            Expr::Call(e) => v.visit_call(e),
            Expr::Closure(e) => v.visit_closure(e),
            Expr::With(e) => v.visit_with(e),
            Expr::Let(e) => v.visit_let(e),
            Expr::If(e) => v.visit_if(e),
            Expr::While(e) => v.visit_while(e),
            Expr::For(e) => v.visit_for(e),
            Expr::Import(e) => v.visit_import(e),
            Expr::Include(e) => v.visit_include(e),
        }
    }

    visit_array(v, array: ArrayExpr) {
        for expr in r!(array.items) {
            v.visit_expr(expr);
        }
    }

    visit_dict(v, dict: DictExpr) {
        for named in r!(dict.items) {
            v.visit_expr(r!(named.expr));
        }
    }

    visit_template(v, template: TemplateExpr) {
        v.visit_enter();
        v.visit_tree(r!(rc: template.tree));
        v.visit_exit();
    }

    visit_group(v, group: GroupExpr) {
        v.visit_expr(r!(group.expr));
    }

    visit_block(v, block: BlockExpr) {
        if block.scoping {
            v.visit_enter();
        }
        for expr in r!(block.exprs) {
            v.visit_expr(expr);
        }
        if block.scoping {
            v.visit_exit();
        }
    }

    visit_binary(v, binary: BinaryExpr) {
        v.visit_expr(r!(binary.lhs));
        v.visit_expr(r!(binary.rhs));
    }

    visit_unary(v, unary: UnaryExpr) {
        v.visit_expr(r!(unary.expr));
    }

    visit_call(v, call: CallExpr) {
        v.visit_expr(r!(call.callee));
        v.visit_args(r!(call.args));
    }

    visit_closure(v, closure: ClosureExpr) {
        for param in r!(rc: closure.params) {
            v.visit_binding(param);
        }
        v.visit_expr(r!(rc: closure.body));
    }

    visit_args(v, args: CallArgs) {
        for arg in r!(args.items) {
            v.visit_arg(arg);
        }
    }

    visit_arg(v, arg: CallArg) {
        match arg {
            CallArg::Pos(expr) => v.visit_expr(expr),
            CallArg::Named(named) => v.visit_expr(r!(named.expr)),
        }
    }

    visit_with(v, with_expr: WithExpr) {
        v.visit_expr(r!(with_expr.callee));
        v.visit_args(r!(with_expr.args));
    }

    visit_let(v, let_expr: LetExpr) {
        if let Some(init) = r!(let_expr.init) {
            v.visit_expr(init);
        }
        v.visit_binding(r!(let_expr.binding));
    }

    visit_if(v, if_expr: IfExpr) {
        v.visit_expr(r!(if_expr.condition));
        v.visit_expr(r!(if_expr.if_body));
        if let Some(body) = r!(if_expr.else_body) {
            v.visit_expr(body);
        }
    }

    visit_while(v, while_expr: WhileExpr) {
        v.visit_expr(r!(while_expr.condition));
        v.visit_expr(r!(while_expr.body));
    }

    visit_for(v, for_expr: ForExpr) {
        v.visit_expr(r!(for_expr.iter));
        match r!(for_expr.pattern) {
            ForPattern::Value(value) => v.visit_binding(value),
            ForPattern::KeyValue(key, value) => {
                v.visit_binding(key);
                v.visit_binding(value);
            }
        }
        v.visit_expr(r!(for_expr.body));
    }

    visit_import(v, import_expr: ImportExpr) {
        v.visit_expr(r!(import_expr.path));
        if let Imports::Idents(idents) = r!(import_expr.imports) {
            for ident in idents {
                v.visit_binding(ident);
            }
        }
    }

    visit_include(v, include_expr: IncludeExpr) {
        v.visit_expr(r!(include_expr.path));
    }
}
