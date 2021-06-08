//! Syntax tree traversal.

use super::*;

macro_rules! visit {
    ($(fn $name:ident($v:ident $(, $node:ident: &$ty:ty)?) $body:block)*) => {
        /// Traverses the syntax tree.
        pub trait Visit<'ast> {
            $(fn $name(&mut self $(, $node: &'ast $ty)?) {
                $name(self, $($node)?);
            })*

            /// Visit a definition of a binding.
            ///
            /// Bindings are, for example, left-hand side of let expressions,
            /// and key/value patterns in for loops.
            fn visit_binding(&mut self, _: &'ast Ident) {}

            /// Visit the entry into a scope.
            fn visit_enter(&mut self) {}

            /// Visit the exit from a scope.
            fn visit_exit(&mut self) {}
        }

        $(visit! {
            @$(concat!("Walk a node of type [`", stringify!($ty), "`]."), )?
            pub fn $name<'ast, V>(
                #[allow(unused)] $v: &mut V
                $(, #[allow(unused)] $node: &'ast $ty)?
            )
            where
                V: Visit<'ast> + ?Sized
            $body
        })*
    };

    (@$doc:expr, $($tts:tt)*) => {
        #[doc = $doc]
        $($tts)*
    };
}

visit! {
    fn visit_tree(v, node: &Tree) {
        for node in node {
            v.visit_node(&node);
        }
    }

    fn visit_node(v, node: &Node) {
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
            Node::Expr(n) => v.visit_expr(n),
        }
    }

    fn visit_heading(v, node: &HeadingNode) {
        v.visit_tree(&node.body);
    }

    fn visit_list(v, node: &ListNode) {
        v.visit_tree(&node.body);
    }

    fn visit_expr(v, node: &Expr) {
        match node {
            Expr::None(_) => {}
            Expr::Bool(_, _) => {}
            Expr::Int(_, _) => {}
            Expr::Float(_, _) => {}
            Expr::Length(_, _, _) => {}
            Expr::Angle(_, _, _) => {}
            Expr::Percent(_, _) => {}
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
            Expr::Let(e) => v.visit_let(e),
            Expr::If(e) => v.visit_if(e),
            Expr::While(e) => v.visit_while(e),
            Expr::For(e) => v.visit_for(e),
            Expr::Import(e) => v.visit_import(e),
            Expr::Include(e) => v.visit_include(e),
        }
    }

    fn visit_array(v, node: &ArrayExpr) {
        for expr in &node.items {
            v.visit_expr(&expr);
        }
    }

    fn visit_dict(v, node: &DictExpr) {
        for named in &node.items {
            v.visit_expr(&named.expr);
        }
    }

    fn visit_template(v, node: &TemplateExpr) {
        v.visit_enter();
        v.visit_tree(&node.tree);
        v.visit_exit();
    }

    fn visit_group(v, node: &GroupExpr) {
        v.visit_expr(&node.expr);
    }

    fn visit_block(v, node: &BlockExpr) {
        if node.scoping {
            v.visit_enter();
        }
        for expr in &node.exprs {
            v.visit_expr(&expr);
        }
        if node.scoping {
            v.visit_exit();
        }
    }

    fn visit_binary(v, node: &BinaryExpr) {
        v.visit_expr(&node.lhs);
        v.visit_expr(&node.rhs);
    }

    fn visit_unary(v, node: &UnaryExpr) {
        v.visit_expr(&node.expr);
    }

    fn visit_call(v, node: &CallExpr) {
        v.visit_expr(&node.callee);
        v.visit_args(&node.args);
    }

    fn visit_closure(v, node: &ClosureExpr) {
        for param in node.params.iter() {
            v.visit_binding(param);
        }
        v.visit_expr(&node.body);
    }

    fn visit_args(v, node: &CallArgs) {
        for arg in &node.items {
            v.visit_arg(arg);
        }
    }

    fn visit_arg(v, node: &CallArg) {
        match node {
            CallArg::Pos(expr) => v.visit_expr(&expr),
            CallArg::Named(named) => v.visit_expr(&named.expr),
        }
    }

    fn visit_let(v, node: &LetExpr) {
        v.visit_binding(&node.binding);
        if let Some(init) = &node.init {
            v.visit_expr(&init);
        }
    }

    fn visit_if(v, node: &IfExpr) {
        v.visit_expr(&node.condition);
        v.visit_expr(&node.if_body);
        if let Some(body) = &node.else_body {
            v.visit_expr(&body);
        }
    }

    fn visit_while(v, node: &WhileExpr) {
        v.visit_expr(&node.condition);
        v.visit_expr(&node.body);
    }

    fn visit_for(v, node: &ForExpr) {
        match &node.pattern {
            ForPattern::Value(value) => v.visit_binding(value),
            ForPattern::KeyValue(key, value) => {
                v.visit_binding(key);
                v.visit_binding(value);
            }
        }
        v.visit_expr(&node.iter);
        v.visit_expr(&node.body);
    }

    fn visit_import(v, node: &ImportExpr) {
        v.visit_expr(&node.path);
        if let Imports::Idents(idents) = &node.imports {
            for ident in idents {
                v.visit_binding(ident);
            }
        }
    }

    fn visit_include(v, node: &IncludeExpr) {
        v.visit_expr(&node.path);
    }
}
