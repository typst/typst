//! Syntax tree traversal.

use super::*;

/// Visits syntax tree nodes in a depth-first manner.
pub trait Visitor<'a>: Sized {
    /// Visit a variable definition.
    fn visit_def(&mut self, _ident: &'a mut Ident) {}

    /// Visit the start of a scope.
    fn visit_scope_pre(&mut self) {}

    /// Visit the end of a scope.
    fn visit_scope_post(&mut self) {}

    fn visit_node(&mut self, node: &'a mut Node) {
        walk_node(self, node)
    }
    fn visit_expr(&mut self, expr: &'a mut Expr) {
        walk_expr(self, expr)
    }
    fn visit_array(&mut self, array: &'a mut ExprArray) {
        walk_array(self, array)
    }
    fn visit_dict(&mut self, dict: &'a mut ExprDict) {
        walk_dict(self, dict)
    }
    fn visit_template(&mut self, template: &'a mut ExprTemplate) {
        walk_template(self, template)
    }
    fn visit_group(&mut self, group: &'a mut ExprGroup) {
        walk_group(self, group)
    }
    fn visit_block(&mut self, block: &'a mut ExprBlock) {
        walk_block(self, block)
    }
    fn visit_binary(&mut self, binary: &'a mut ExprBinary) {
        walk_binary(self, binary)
    }
    fn visit_unary(&mut self, unary: &'a mut ExprUnary) {
        walk_unary(self, unary)
    }
    fn visit_call(&mut self, call: &'a mut ExprCall) {
        walk_call(self, call)
    }
    fn visit_arg(&mut self, arg: &'a mut Argument) {
        walk_arg(self, arg)
    }
    fn visit_let(&mut self, expr_let: &'a mut ExprLet) {
        walk_let(self, expr_let)
    }
    fn visit_if(&mut self, expr_if: &'a mut ExprIf) {
        walk_if(self, expr_if)
    }
    fn visit_for(&mut self, expr_for: &'a mut ExprFor) {
        walk_for(self, expr_for)
    }
}

pub fn walk_node<'a, V: Visitor<'a>>(v: &mut V, node: &'a mut Node) {
    match node {
        Node::Strong => {}
        Node::Emph => {}
        Node::Space => {}
        Node::Linebreak => {}
        Node::Parbreak => {}
        Node::Text(_) => {}
        Node::Heading(_) => {}
        Node::Raw(_) => {}
        Node::Expr(expr) => v.visit_expr(expr),
    }
}

pub fn walk_expr<'a, V: Visitor<'a>>(v: &mut V, expr: &'a mut Expr) {
    match expr {
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

pub fn walk_array<'a, V: Visitor<'a>>(v: &mut V, array: &'a mut ExprArray) {
    for expr in array {
        v.visit_expr(&mut expr.v);
    }
}

pub fn walk_dict<'a, V: Visitor<'a>>(v: &mut V, dict: &'a mut ExprDict) {
    for named in dict {
        v.visit_expr(&mut named.expr.v);
    }
}

pub fn walk_template<'a, V: Visitor<'a>>(v: &mut V, template: &'a mut ExprTemplate) {
    v.visit_scope_pre();
    for node in template {
        v.visit_node(&mut node.v);
    }
    v.visit_scope_post();
}

pub fn walk_group<'a, V: Visitor<'a>>(v: &mut V, group: &'a mut ExprGroup) {
    v.visit_expr(&mut group.v);
}

pub fn walk_block<'a, V: Visitor<'a>>(v: &mut V, block: &'a mut ExprBlock) {
    if block.scopes {
        v.visit_scope_pre();
    }
    for expr in &mut block.exprs {
        v.visit_expr(&mut expr.v);
    }
    if block.scopes {
        v.visit_scope_post();
    }
}

pub fn walk_binary<'a, V: Visitor<'a>>(v: &mut V, binary: &'a mut ExprBinary) {
    v.visit_expr(&mut binary.lhs.v);
    v.visit_expr(&mut binary.rhs.v);
}

pub fn walk_unary<'a, V: Visitor<'a>>(v: &mut V, unary: &'a mut ExprUnary) {
    v.visit_expr(&mut unary.expr.v);
}

pub fn walk_call<'a, V: Visitor<'a>>(v: &mut V, call: &'a mut ExprCall) {
    v.visit_expr(&mut call.callee.v);
    for arg in &mut call.args.v {
        v.visit_arg(arg);
    }
}

pub fn walk_arg<'a, V: Visitor<'a>>(v: &mut V, arg: &'a mut Argument) {
    match arg {
        Argument::Pos(expr) => v.visit_expr(&mut expr.v),
        Argument::Named(named) => v.visit_expr(&mut named.expr.v),
    }
}

pub fn walk_let<'a, V: Visitor<'a>>(v: &mut V, expr_let: &'a mut ExprLet) {
    v.visit_def(&mut expr_let.pat.v);
    if let Some(init) = &mut expr_let.init {
        v.visit_expr(&mut init.v);
    }
}

pub fn walk_if<'a, V: Visitor<'a>>(v: &mut V, expr_if: &'a mut ExprIf) {
    v.visit_expr(&mut expr_if.condition.v);
    v.visit_expr(&mut expr_if.if_body.v);
    if let Some(body) = &mut expr_if.else_body {
        v.visit_expr(&mut body.v);
    }
}

pub fn walk_for<'a, V: Visitor<'a>>(v: &mut V, expr_for: &'a mut ExprFor) {
    match &mut expr_for.pat.v {
        ForPattern::Value(value) => v.visit_def(value),
        ForPattern::KeyValue(key, value) => {
            v.visit_def(key);
            v.visit_def(value);
        }
    }
    v.visit_expr(&mut expr_for.iter.v);
    v.visit_expr(&mut expr_for.body.v);
}
