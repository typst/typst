//! Syntax tree traversal.

use super::*;

macro_rules! visit {
    ($(fn $name:ident($v:ident, $item:ident: &mut $ty:ty) $body:block)*) => {
        /// Traverses the syntax tree.
        pub trait Visit<'ast> {
            $(fn $name(&mut self, $item: &'ast mut $ty) {
                $name(self, $item);
            })*
        }

        $(visit! {
            @concat!("Walk a node of type [`", stringify!($ty), "`]."),
            pub fn $name<'ast, V>($v: &mut V, $item: &'ast mut $ty)
            where
                V: Visit<'ast> + ?Sized
            $body
        })*
    };
    (@$doc:expr, $($tts:tt)*) => {
        #[doc = $doc]
        $($tts)*
    }

}

visit! {
    fn visit_tree(v, item: &mut Tree) {
        for node in item {
            v.visit_node(&mut node.v);
        }
    }

    fn visit_node(v, item: &mut Node) {
        match item {
            Node::Strong => {}
            Node::Emph => {}
            Node::Space => {}
            Node::Linebreak => {}
            Node::Parbreak => {}
            Node::Text(_) => {}
            Node::Heading(n) => v.visit_tree(&mut n.contents),
            Node::Raw(_) => {}
            Node::Expr(expr) => v.visit_expr(expr),
        }
    }

    fn visit_expr(v, item: &mut Expr) {
        match item {
            Expr::None => {}
            Expr::Ident(_) => {}
            Expr::Bool(_) => {}
            Expr::Int(_) => {}
            Expr::Float(_) => {}
            Expr::Length(_, _) => {}
            Expr::Angle(_, _) => {}
            Expr::Percent(_) => {}
            Expr::Color(_) => {}
            Expr::Str(_) => {}
            Expr::Array(e) => v.visit_array(e),
            Expr::Dict(e) => v.visit_dict(e),
            Expr::Template(e) => v.visit_template(e),
            Expr::Group(e) => v.visit_group(e),
            Expr::Block(e) => v.visit_block(e),
            Expr::Unary(e) => v.visit_unary(e),
            Expr::Binary(e) => v.visit_binary(e),
            Expr::Call(e) => v.visit_call(e),
            Expr::Let(e) => v.visit_let(e),
            Expr::If(e) => v.visit_if(e),
            Expr::For(e) => v.visit_for(e),
            Expr::Captured(_) => {}
        }
    }

    fn visit_array(v, item: &mut ExprArray) {
        for expr in item {
            v.visit_expr(&mut expr.v);
        }
    }

    fn visit_dict(v, item: &mut ExprDict) {
        for named in item {
            v.visit_expr(&mut named.expr.v);
        }
    }

    fn visit_template(v, item: &mut ExprTemplate) {
        v.visit_tree(item);
    }

    fn visit_group(v, item: &mut ExprGroup) {
        v.visit_expr(&mut item.v);
    }

    fn visit_block(v, item: &mut ExprBlock) {
        for expr in &mut item.exprs {
            v.visit_expr(&mut expr.v);
        }
    }

    fn visit_binary(v, item: &mut ExprBinary) {
        v.visit_expr(&mut item.lhs.v);
        v.visit_expr(&mut item.rhs.v);
    }

    fn visit_unary(v, item: &mut ExprUnary) {
        v.visit_expr(&mut item.expr.v);
    }

    fn visit_call(v, item: &mut ExprCall) {
        v.visit_expr(&mut item.callee.v);
        v.visit_args(&mut item.args.v);
    }

    fn visit_args(v, item: &mut ExprArgs) {
        for arg in item {
            v.visit_arg(arg);
        }
    }

    fn visit_arg(v, item: &mut Argument) {
        match item {
            Argument::Pos(expr) => v.visit_expr(&mut expr.v),
            Argument::Named(named) => v.visit_expr(&mut named.expr.v),
        }
    }

    fn visit_let(v, item: &mut ExprLet) {
        if let Some(init) = &mut item.init {
            v.visit_expr(&mut init.v);
        }
    }

    fn visit_if(v, item: &mut ExprIf) {
        v.visit_expr(&mut item.condition.v);
        v.visit_expr(&mut item.if_body.v);
        if let Some(body) = &mut item.else_body {
            v.visit_expr(&mut body.v);
        }
    }

    fn visit_for(v, item: &mut ExprFor) {
        v.visit_expr(&mut item.iter.v);
        v.visit_expr(&mut item.body.v);
    }
}
