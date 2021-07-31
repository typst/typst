//! Pretty printing.

use std::fmt::{self, Arguments, Write};

use crate::color::{Color, RgbaColor};
use crate::eval::*;
use crate::geom::{Angle, Fractional, Length, Linear, Relative};
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

/// Pretty print an item.
pub trait Pretty {
    /// Pretty print this item into the given printer.
    fn pretty(&self, p: &mut Printer);
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

impl Pretty for SyntaxTree {
    fn pretty(&self, p: &mut Printer) {
        for node in self {
            node.pretty(p);
        }
    }
}

impl Pretty for SyntaxNode {
    fn pretty(&self, p: &mut Printer) {
        match self {
            // TODO: Handle escaping.
            Self::Text(text) => p.push_str(text),
            Self::Space => p.push(' '),
            Self::Linebreak(_) => p.push_str(r"\"),
            Self::Parbreak(_) => p.push_str("\n\n"),
            Self::Strong(_) => p.push('*'),
            Self::Emph(_) => p.push('_'),
            Self::Raw(raw) => raw.pretty(p),
            Self::Heading(n) => n.pretty(p),
            Self::List(n) => n.pretty(p),
            Self::Enum(n) => n.pretty(p),
            Self::Expr(n) => {
                if n.has_short_form() {
                    p.push('#');
                }
                n.pretty(p);
            }
        }
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
        // backticks.
        let mut count = 0;
        for c in self.text.chars() {
            if c == '`' {
                count += 1;
                backticks = backticks.max(3).max(count + 1);
            } else {
                count = 0;
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
        p.push_str(&self.text);

        // End untrimming.
        if self.block {
            p.push('\n');
        } else if self.text.trim_end().ends_with('`') {
            p.push(' ');
        }

        // Ending backticks.
        for _ in 0 .. backticks {
            p.push('`');
        }
    }
}

impl Pretty for HeadingNode {
    fn pretty(&self, p: &mut Printer) {
        for _ in 0 .. self.level {
            p.push('=');
        }
        p.push(' ');
        self.body.pretty(p);
    }
}

impl Pretty for ListItem {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("- ");
        self.body.pretty(p);
    }
}

impl Pretty for EnumItem {
    fn pretty(&self, p: &mut Printer) {
        if let Some(number) = self.number {
            write!(p, "{}", number).unwrap();
        }
        p.push_str(". ");
        self.body.pretty(p);
    }
}

impl Pretty for Expr {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::None(_) => p.push_str("none"),
            Self::Auto(_) => p.push_str("auto"),
            Self::Bool(_, v) => v.pretty(p),
            Self::Int(_, v) => v.pretty(p),
            Self::Float(_, v) => v.pretty(p),
            Self::Length(_, v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Angle(_, v, u) => write!(p, "{}{}", v, u).unwrap(),
            Self::Percent(_, v) => write!(p, "{}%", v).unwrap(),
            Self::Fractional(_, v) => write!(p, "{}fr", v).unwrap(),
            Self::Str(_, v) => v.pretty(p),
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
            Self::With(v) => v.pretty(p),
            Self::Let(v) => v.pretty(p),
            Self::If(v) => v.pretty(p),
            Self::While(v) => v.pretty(p),
            Self::For(v) => v.pretty(p),
            Self::Import(v) => v.pretty(p),
            Self::Include(v) => v.pretty(p),
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
        self.tree.pretty(p);
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

impl Pretty for WithExpr {
    fn pretty(&self, p: &mut Printer) {
        self.callee.pretty(p);
        p.push_str(" with (");
        self.args.pretty(p);
        p.push(')');
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

impl Pretty for ImportExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("import ");
        self.imports.pretty(p);
        p.push_str(" from ");
        self.path.pretty(p);
    }
}

impl Pretty for Imports {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Wildcard => p.push('*'),
            Self::Idents(idents) => p.join(idents, ", ", |item, p| item.pretty(p)),
        }
    }
}

impl Pretty for IncludeExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("include ");
        self.path.pretty(p);
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
            Self::None => p.push_str("none"),
            Self::Auto => p.push_str("auto"),
            Self::Bool(v) => v.pretty(p),
            Self::Int(v) => v.pretty(p),
            Self::Float(v) => v.pretty(p),
            Self::Length(v) => v.pretty(p),
            Self::Angle(v) => v.pretty(p),
            Self::Relative(v) => v.pretty(p),
            Self::Linear(v) => v.pretty(p),
            Self::Fractional(v) => v.pretty(p),
            Self::Color(v) => v.pretty(p),
            Self::Str(v) => v.pretty(p),
            Self::Array(v) => v.pretty(p),
            Self::Dict(v) => v.pretty(p),
            Self::Template(v) => v.pretty(p),
            Self::Func(v) => v.pretty(p),
            Self::Dyn(v) => v.pretty(p),
        }
    }
}

impl Pretty for Array {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        p.join(self, ", ", |item, p| item.pretty(p));
        if self.len() == 1 {
            p.push(',');
        }
        p.push(')');
    }
}

impl Pretty for Dict {
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

impl Pretty for Template {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("<template>");
    }
}

impl Pretty for Function {
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
            p.push_str(&name);
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
    Fractional,
    RgbaColor,
    Color,
    Dynamic,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loading::FileId;
    use crate::parse::parse;
    use crate::source::SourceFile;

    #[track_caller]
    fn roundtrip(src: &str) {
        test_parse(src, src);
    }

    #[track_caller]
    fn test_parse(src: &str, exp: &str) {
        let source = SourceFile::new(FileId::from_raw(0), src.into());
        let ast = parse(&source).unwrap();
        let found = pretty(&ast);
        if exp != found {
            println!("tree:     {:#?}", ast);
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
        roundtrip("= *Ok*");
        roundtrip("- Ok");

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
        roundtrip("{auto}");
        roundtrip("{true}");
        roundtrip("{10}");
        roundtrip("{3.14}");
        roundtrip("{10pt}");
        roundtrip("{14.1deg}");
        roundtrip("{20%}");
        roundtrip("{0.5fr}");
        roundtrip(r#"{"hi"}"#);
        test_parse(r#"{"let's \" go"}"#, r#"{"let's \" go"}"#);
        roundtrip("{hi}");

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

        // Functions.
        roundtrip("{v()}");
        roundtrip("{v()()}");
        roundtrip("{v(1)}");
        roundtrip("{v(a: 1, b)}");
        roundtrip("#v()");
        roundtrip("#v(1)");
        roundtrip("#v(1, 2)[*Ok*]");
        roundtrip("#v(1, f[2])");
        roundtrip("{(a, b) => a + b}");

        // Control flow.
        roundtrip("#let x = 1 + 2");
        test_parse("#let f(x) = y", "#let f = (x) => y");
        test_parse("#if x [y] #else [z]", "#if x [y] else [z]");
        roundtrip("#while x {y}");
        roundtrip("#for x in y {z}");
        roundtrip("#for k, x in y {z}");
        roundtrip("#import * from \"file.typ\"");
        roundtrip("#include \"chapter1.typ\"");
    }

    #[test]
    fn test_pretty_print_value() {
        // Primitives.
        test_value(Value::None, "none");
        test_value(false, "false");
        test_value(12i64, "12");
        test_value(3.14, "3.14");
        test_value(Length::pt(5.5), "5.5pt");
        test_value(Angle::deg(90.0), "90deg");
        test_value(Relative::one() / 2.0, "50%");
        test_value(Relative::new(0.3) + Length::cm(2.0), "30% + 2cm");
        test_value(Fractional::one() * 7.55, "7.55fr");
        test_value(Color::Rgba(RgbaColor::new(1, 1, 1, 0xff)), "#010101");

        // Collections.
        test_value("hello", r#""hello""#);
        test_value("\n", r#""\n""#);
        test_value("\\", r#""\\""#);
        test_value("\"", r#""\"""#);
        test_value(array![], "()");
        test_value(array![Value::None], "(none,)");
        test_value(array![1, 2], "(1, 2)");
        test_value(dict![], "(:)");
        test_value(dict!["one" => 1], "(one: 1)");
        test_value(dict!["two" => false, "one" => 1], "(one: 1, two: false)");

        // Functions.
        test_value(Function::new(None, |_, _| Ok(Value::None)), "<function>");
        test_value(
            Function::new(Some("nil".into()), |_, _| Ok(Value::None)),
            "<function nil>",
        );

        // Dynamics.
        test_value(Dynamic::new(1), "1");
    }
}
