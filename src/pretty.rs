//! Pretty printing.

use std::fmt::{self, Arguments, Write};

use crate::color::{Color, RgbaColor};
use crate::eval::*;
use crate::geom::{Angle, Length, Linear, Relative};
use crate::syntax::*;

/// Pretty print an item and return the resulting string.
pub fn pretty<T>(item: &T) -> String
where
    T: Pretty + ?Sized,
{
    let mut p = Printer::new();
    item.pretty(&mut p);
    p.finish()
}

/// Pretty print an item with an expression map and return the resulting string.
pub fn pretty_with_map<T>(item: &T, map: &ExprMap) -> String
where
    T: PrettyWithMap + ?Sized,
{
    let mut p = Printer::new();
    item.pretty_with_map(&mut p, Some(map));
    p.finish()
}

/// Pretty print an item.
pub trait Pretty {
    /// Pretty print this item into the given printer.
    fn pretty(&self, p: &mut Printer);
}

/// Pretty print an item with an expression map that applies to it.
pub trait PrettyWithMap {
    /// Pretty print this item into the given printer.
    fn pretty_with_map(&self, p: &mut Printer, map: Option<&ExprMap>);
}

impl<T> Pretty for T
where
    T: PrettyWithMap,
{
    fn pretty(&self, p: &mut Printer) {
        self.pretty_with_map(p, None);
    }
}

/// A buffer into which items are printed.
pub struct Printer {
    buf: String,
}

impl Printer {
    /// Create a new pretty printer.
    pub fn new() -> Self {
        Self { buf: String::new() }
    }

    /// Push a character into the buffer.
    pub fn push(&mut self, c: char) {
        self.buf.push(c);
    }

    /// Push a string into the buffer.
    pub fn push_str(&mut self, string: &str) {
        self.buf.push_str(string);
    }

    /// Write formatted items into the buffer.
    pub fn write_fmt(&mut self, fmt: Arguments<'_>) -> fmt::Result {
        Write::write_fmt(self, fmt)
    }

    /// Write a list of items joined by a joiner.
    pub fn join<T, I, F>(&mut self, items: I, joiner: &str, mut write_item: F)
    where
        I: IntoIterator<Item = T>,
        F: FnMut(T, &mut Self),
    {
        let mut iter = items.into_iter();
        if let Some(first) = iter.next() {
            write_item(first, self);
        }
        for item in iter {
            self.push_str(joiner);
            write_item(item, self);
        }
    }

    /// Finish pretty printing and return the underlying buffer.
    pub fn finish(self) -> String {
        self.buf
    }
}

impl Write for Printer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.push_str(s);
        Ok(())
    }
}

impl PrettyWithMap for Tree {
    fn pretty_with_map(&self, p: &mut Printer, map: Option<&ExprMap>) {
        for node in self {
            node.pretty_with_map(p, map);
        }
    }
}

impl PrettyWithMap for Node {
    fn pretty_with_map(&self, p: &mut Printer, map: Option<&ExprMap>) {
        match self {
            Self::Strong => p.push('*'),
            Self::Emph => p.push('_'),
            Self::Space => p.push(' '),
            Self::Linebreak => p.push_str(r"\"),
            Self::Parbreak => p.push_str("\n\n"),
            // TODO: Handle escaping.
            Self::Text(text) => p.push_str(text),
            Self::Heading(heading) => heading.pretty_with_map(p, map),
            Self::Raw(raw) => raw.pretty(p),
            Self::Expr(expr) => {
                if let Some(map) = map {
                    let value = &map[&(expr as *const _)];
                    value.pretty(p);
                } else {
                    if expr.has_short_form() {
                        p.push('#');
                    }
                    expr.pretty(p);
                }
            }
        }
    }
}

impl PrettyWithMap for HeadingNode {
    fn pretty_with_map(&self, p: &mut Printer, map: Option<&ExprMap>) {
        for _ in 0 ..= self.level {
            p.push('=');
        }
        self.contents.pretty_with_map(p, map);
    }
}

impl Pretty for RawNode {
    fn pretty(&self, p: &mut Printer) {
        // Find out how many backticks we need.
        let mut backticks = 1;

        // Language tag and block-level are only possible with 3+ backticks.
        if self.lang.is_some() || self.block {
            backticks = 3;
        }

        // More backticks may be required if there are lots of consecutive
        // backticks in the lines.
        let mut count;
        for line in &self.lines {
            count = 0;
            for c in line.chars() {
                if c == '`' {
                    count += 1;
                    backticks = backticks.max(3).max(count + 1);
                } else {
                    count = 0;
                }
            }
        }

        // Starting backticks.
        for _ in 0 .. backticks {
            p.push('`');
        }

        // Language tag.
        if let Some(lang) = &self.lang {
            lang.pretty(p);
        }

        // Start untrimming.
        if self.block {
            p.push('\n');
        } else if backticks >= 3 {
            p.push(' ');
        }

        // The lines.
        p.join(&self.lines, "\n", |line, p| p.push_str(line));

        // End untrimming.
        if self.block {
            p.push('\n');
        } else if self.lines.last().map_or(false, |line| line.trim_end().ends_with('`')) {
            p.push(' ');
        }

        // Ending backticks.
        for _ in 0 .. backticks {
            p.push('`');
        }
    }
}

impl Pretty for Expr {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Lit(v) => v.pretty(p),
            Self::Ident(v) => v.pretty(p),
            Self::Array(v) => v.pretty(p),
            Self::Dict(v) => v.pretty(p),
            Self::Template(v) => v.pretty(p),
            Self::Group(v) => v.pretty(p),
            Self::Block(v) => v.pretty(p),
            Self::Unary(v) => v.pretty(p),
            Self::Binary(v) => v.pretty(p),
            Self::Call(v) => v.pretty(p),
            Self::Closure(v) => v.pretty(p),
            Self::Let(v) => v.pretty(p),
            Self::If(v) => v.pretty(p),
            Self::While(v) => v.pretty(p),
            Self::For(v) => v.pretty(p),
        }
    }
}

impl Pretty for Lit {
    fn pretty(&self, p: &mut Printer) {
        self.kind.pretty(p);
    }
}

impl Pretty for LitKind {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::None => p.push_str("none"),
            Self::Bool(v) => v.pretty(p),
            Self::Int(v) => v.pretty(p),
            Self::Float(v) => v.pretty(p),
            Self::Length(v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Angle(v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Percent(v) => write!(p, "{}%", v).unwrap(),
            Self::Color(v) => v.pretty(p),
            Self::Str(v) => v.pretty(p),
        }
    }
}

impl Pretty for ArrayExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        p.join(&self.items, ", ", |item, p| item.pretty(p));
        if self.items.len() == 1 {
            p.push(',');
        }
        p.push(')');
    }
}

impl Pretty for DictExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        if self.items.is_empty() {
            p.push(':');
        } else {
            p.join(&self.items, ", ", |named, p| named.pretty(p));
        }
        p.push(')');
    }
}

impl Pretty for Named {
    fn pretty(&self, p: &mut Printer) {
        self.name.pretty(p);
        p.push_str(": ");
        self.expr.pretty(p);
    }
}

impl Pretty for TemplateExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('[');
        self.tree.pretty_with_map(p, None);
        p.push(']');
    }
}

impl Pretty for GroupExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        self.expr.pretty(p);
        p.push(')');
    }
}

impl Pretty for BlockExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('{');
        if self.exprs.len() > 1 {
            p.push(' ');
        }
        p.join(&self.exprs, "; ", |expr, p| expr.pretty(p));
        if self.exprs.len() > 1 {
            p.push(' ');
        }
        p.push('}');
    }
}

impl Pretty for UnaryExpr {
    fn pretty(&self, p: &mut Printer) {
        self.op.pretty(p);
        if self.op == UnOp::Not {
            p.push(' ');
        }
        self.expr.pretty(p);
    }
}

impl Pretty for UnOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

impl Pretty for BinaryExpr {
    fn pretty(&self, p: &mut Printer) {
        self.lhs.pretty(p);
        p.push(' ');
        self.op.pretty(p);
        p.push(' ');
        self.rhs.pretty(p);
    }
}

impl Pretty for BinOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

impl Pretty for CallExpr {
    fn pretty(&self, p: &mut Printer) {
        self.callee.pretty(p);

        let mut write_args = |items: &[CallArg]| {
            p.push('(');
            p.join(items, ", ", |item, p| item.pretty(p));
            p.push(')');
        };

        match self.args.items.as_slice() {
            // This can be moved behind the arguments.
            //
            // Example: Transforms "#v(a, [b])" => "#v(a)[b]".
            [head @ .., CallArg::Pos(Expr::Template(template))] => {
                if !head.is_empty() {
                    write_args(head);
                }
                template.pretty(p);
            }

            items => write_args(items),
        }
    }
}

impl Pretty for CallArgs {
    fn pretty(&self, p: &mut Printer) {
        p.join(&self.items, ", ", |item, p| item.pretty(p));
    }
}

impl Pretty for CallArg {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Pos(expr) => expr.pretty(p),
            Self::Named(named) => named.pretty(p),
        }
    }
}

impl Pretty for ClosureExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        p.join(self.params.iter(), ", ", |item, p| item.pretty(p));
        p.push_str(") => ");
        self.body.pretty(p);
    }
}

impl Pretty for LetExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("let ");
        self.binding.pretty(p);
        if let Some(init) = &self.init {
            p.push_str(" = ");
            init.pretty(p);
        }
    }
}

impl Pretty for IfExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("if ");
        self.condition.pretty(p);
        p.push(' ');
        self.if_body.pretty(p);
        if let Some(expr) = &self.else_body {
            // FIXME: Hashtag in markup.
            p.push_str(" else ");
            expr.pretty(p);
        }
    }
}

impl Pretty for WhileExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("while ");
        self.condition.pretty(p);
        p.push(' ');
        self.body.pretty(p);
    }
}

impl Pretty for ForExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("for ");
        self.pattern.pretty(p);
        p.push_str(" in ");
        self.iter.pretty(p);
        p.push(' ');
        self.body.pretty(p);
    }
}

impl Pretty for ForPattern {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Value(v) => v.pretty(p),
            Self::KeyValue(k, v) => {
                k.pretty(p);
                p.push_str(", ");
                v.pretty(p);
            }
        }
    }
}

impl Pretty for Ident {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

impl Pretty for Value {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Value::None => p.push_str("none"),
            Value::Bool(v) => v.pretty(p),
            Value::Int(v) => v.pretty(p),
            Value::Float(v) => v.pretty(p),
            Value::Length(v) => v.pretty(p),
            Value::Angle(v) => v.pretty(p),
            Value::Relative(v) => v.pretty(p),
            Value::Linear(v) => v.pretty(p),
            Value::Color(v) => v.pretty(p),
            // TODO: Handle like text when directly in template.
            Value::Str(v) => v.pretty(p),
            Value::Array(v) => v.pretty(p),
            Value::Dict(v) => v.pretty(p),
            Value::Template(v) => v.pretty(p),
            Value::Func(v) => v.pretty(p),
            Value::Any(v) => v.pretty(p),
            Value::Error => p.push_str("<error>"),
        }
    }
}

impl Pretty for ArrayValue {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        p.join(self, ", ", |item, p| item.pretty(p));
        if self.len() == 1 {
            p.push(',');
        }
        p.push(')');
    }
}

impl Pretty for DictValue {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        if self.is_empty() {
            p.push(':');
        } else {
            p.join(self, ", ", |(key, value), p| {
                p.push_str(key);
                p.push_str(": ");
                value.pretty(p);
            });
        }
        p.push(')');
    }
}

impl Pretty for TemplateValue {
    fn pretty(&self, p: &mut Printer) {
        p.push('[');
        for part in self {
            part.pretty(p);
        }
        p.push(']');
    }
}

impl Pretty for TemplateNode {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Tree { tree, map } => tree.pretty_with_map(p, Some(map)),
            Self::Str(s) => p.push_str(s),
            Self::Func(func) => func.pretty(p),
        }
    }
}

impl Pretty for TemplateFunc {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("<node ");
        p.push_str(self.name());
        p.push('>');
    }
}

impl Pretty for FuncValue {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("<function");
        if let Some(name) = self.name() {
            p.push(' ');
            p.push_str(name);
        }
        p.push('>');
    }
}

impl Pretty for FuncArgs {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        p.join(&self.items, ", ", |item, p| item.pretty(p));
        p.push(')');
    }
}

impl Pretty for FuncArg {
    fn pretty(&self, p: &mut Printer) {
        if let Some(name) = &self.name {
            p.push_str(&name.v);
            p.push_str(": ");
        }
        self.value.v.pretty(p);
    }
}

impl Pretty for i64 {
    fn pretty(&self, p: &mut Printer) {
        write!(p, "{}", self).unwrap();
    }
}

impl Pretty for f64 {
    fn pretty(&self, p: &mut Printer) {
        write!(p, "{}", self).unwrap();
    }
}

impl Pretty for str {
    fn pretty(&self, p: &mut Printer) {
        p.push('"');
        for c in self.chars() {
            match c {
                '\\' => p.push_str(r"\\"),
                '"' => p.push_str(r#"\""#),
                '\n' => p.push_str(r"\n"),
                '\r' => p.push_str(r"\r"),
                '\t' => p.push_str(r"\t"),
                _ => p.push(c),
            }
        }
        p.push('"');
    }
}

macro_rules! pretty_display {
    ($($type:ty),* $(,)?) => {
        $(impl Pretty for $type {
            fn pretty(&self, p: &mut Printer) {
                write!(p, "{}", self).unwrap();
            }
        })*
    };
}

pretty_display! {
    bool,
    Length,
    Angle,
    Relative,
    Linear,
    RgbaColor,
    Color,
    AnyValue,
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};
    use std::rc::Rc;

    use super::*;
    use crate::env::Env;
    use crate::parse::parse;

    #[track_caller]
    fn roundtrip(src: &str) {
        test_parse(src, src);
    }

    #[track_caller]
    fn test_parse(src: &str, exp: &str) {
        let tree = parse(src).output;
        let found = pretty(&tree);
        if exp != found {
            println!("tree:     {:#?}", tree);
            println!("expected: {}", exp);
            println!("found:    {}", found);
            panic!("test failed");
        }
    }

    #[track_caller]
    fn test_value(value: impl Into<Value>, exp: &str) {
        assert_eq!(pretty(&value.into()), exp);
    }

    #[test]
    fn test_pretty_print_node() {
        // Basic text and markup.
        roundtrip("*");
        roundtrip("_");
        roundtrip(" ");
        roundtrip("\\ ");
        roundtrip("\n\n");
        roundtrip("hi");

        // Heading.
        roundtrip("= *Ok*");

        // Raw.
        roundtrip("``");
        roundtrip("`nolang 1`");
        roundtrip("```lang 1```");
        roundtrip("```lang 1 ```");
        roundtrip("```hi  line  ```");
        roundtrip("```py\ndef\n```");
        roundtrip("```\n line \n```");
        roundtrip("```\n`\n```");
        roundtrip("``` ` ```");
        roundtrip("````\n```\n```\n````");
        test_parse("```lang```", "```lang ```");
        test_parse("```1 ```", "``");
        test_parse("``` 1```", "`1`");
        test_parse("``` 1 ```", "`1 `");
        test_parse("```` ` ````", "``` ` ```");
    }

    #[test]
    fn test_pretty_print_expr() {
        // Basic expressions.
        roundtrip("{none}");
        roundtrip("{hi}");
        roundtrip("{true}");
        roundtrip("{10}");
        roundtrip("{3.14}");
        roundtrip("{10pt}");
        roundtrip("{14.1deg}");
        roundtrip("{20%}");
        roundtrip("{#abcdef}");
        roundtrip(r#"{"hi"}"#);
        test_parse(r#"{"let's \" go"}"#, r#"{"let's \" go"}"#);

        // Arrays.
        roundtrip("{()}");
        roundtrip("{(1)}");
        roundtrip("{(1, 2, 3)}");

        // Dictionaries.
        roundtrip("{(:)}");
        roundtrip("{(key: value)}");
        roundtrip("{(a: 1, b: 2)}");

        // Templates.
        roundtrip("[]");
        roundtrip("[*Ok*]");
        roundtrip("{[f]}");

        // Groups.
        roundtrip("{(1)}");

        // Blocks.
        roundtrip("{}");
        roundtrip("{1}");
        roundtrip("{ let x = 1; x += 2; x + 1 }");
        roundtrip("[{}]");

        // Operators.
        roundtrip("{-x}");
        roundtrip("{not true}");
        roundtrip("{1 + 3}");

        // Function calls.
        roundtrip("{v()}");
        roundtrip("{v()()}");
        roundtrip("{v(1)}");
        roundtrip("{v(a: 1, b)}");
        roundtrip("#v()");
        roundtrip("#v(1)");
        roundtrip("#v(1, 2)[*Ok*]");
        roundtrip("#v(1, f[2])");

        // Closures.
        roundtrip("{(a, b) => a + b}");

        // Keywords.
        roundtrip("#let x = 1 + 2");
        test_parse("#let f(x) = y", "#let f = (x) => y");
        test_parse("#if x [y] #else [z]", "#if x [y] else [z]");
        roundtrip("#while x {y}");
        roundtrip("#for x in y {z}");
        roundtrip("#for k, x in y {z}");
    }

    #[test]
    fn test_pretty_print_with_map() {
        let tree = parse("*[{1+2}[{4}]]*{2+3}").output;
        let map = eval(&mut Env::blank(), &tree, &Default::default()).output;
        assert_eq!(pretty_with_map(&tree, &map), "*[3[4]]*5");
    }

    #[test]
    fn test_pretty_print_value() {
        // Simple values.
        test_value(Value::None, "none");
        test_value(false, "false");
        test_value(12i64, "12");
        test_value(3.14, "3.14");
        test_value(Length::pt(5.5), "5.5pt");
        test_value(Angle::deg(90.0), "90deg");
        test_value(Relative::ONE / 2.0, "50%");
        test_value(Relative::new(0.3) + Length::cm(2.0), "30% + 2cm");
        test_value(Color::Rgba(RgbaColor::new(1, 1, 1, 0xff)), "#010101");
        test_value("hello", r#""hello""#);
        test_value("\n", r#""\n""#);
        test_value("\\", r#""\\""#);
        test_value("\"", r#""\"""#);

        // Array.
        test_value(Value::Array(vec![]), "()");
        test_value(vec![Value::None], "(none,)");
        test_value(vec![Value::Int(1), Value::Int(2)], "(1, 2)");

        // Dictionary.
        let mut dict = BTreeMap::new();
        test_value(dict.clone(), "(:)");
        dict.insert("one".into(), Value::Int(1));
        test_value(dict.clone(), "(one: 1)");
        dict.insert("two".into(), Value::Bool(false));
        test_value(dict, "(one: 1, two: false)");

        // Template.
        test_value(
            vec![
                TemplateNode::Tree {
                    tree: Rc::new(vec![Node::Strong]),
                    map: HashMap::new(),
                },
                TemplateNode::Func(TemplateFunc::new("example", |_| {})),
            ],
            "[*<node example>]",
        );

        // Function.
        test_value(FuncValue::new(None, |_, _| Value::None), "<function>");
        test_value(
            FuncValue::new(Some("nil".into()), |_, _| Value::None),
            "<function nil>",
        );

        // Any.
        test_value(AnyValue::new(1), "1");

        // Error.
        test_value(Value::Error, "<error>");
    }

    #[test]
    fn test_pretty_print_args() {
        // Arguments.
        assert_eq!(
            pretty(&FuncArgs {
                span: Span::ZERO,
                items: vec![
                    FuncArg {
                        name: Some(Spanned::zero("a".into())),
                        value: Spanned::zero(Value::Int(1)),
                    },
                    FuncArg {
                        name: None,
                        value: Spanned::zero(Value::Int(2)),
                    },
                ],
            }),
            "(a: 1, 2)",
        );
    }
}
