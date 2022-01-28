//! Pretty printing.

use std::fmt::{self, Arguments, Write};

use super::ast::*;

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

/// A buffer into which items can be pretty printed.
#[derive(Default)]
pub struct Printer {
    buf: String,
}

impl Printer {
    /// Create a new pretty printer.
    pub fn new() -> Self {
        Self::default()
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

    /// Write a list of items joined by a joiner and return how many there were.
    pub fn join<T, I, F>(&mut self, items: I, joiner: &str, mut write_item: F) -> usize
    where
        I: IntoIterator<Item = T>,
        F: FnMut(T, &mut Self),
    {
        let mut count = 0;
        let mut iter = items.into_iter();
        if let Some(first) = iter.next() {
            write_item(first, self);
            count += 1;
        }
        for item in iter {
            self.push_str(joiner);
            write_item(item, self);
            count += 1;
        }
        count
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

impl Pretty for Markup {
    fn pretty(&self, p: &mut Printer) {
        for node in self.nodes() {
            node.pretty(p);
        }
    }
}

impl Pretty for MarkupNode {
    fn pretty(&self, p: &mut Printer) {
        match self {
            // TODO: Handle escaping.
            Self::Space => p.push(' '),
            Self::Linebreak => p.push_str(r"\"),
            Self::Parbreak => p.push_str("\n\n"),
            Self::Strong => p.push('*'),
            Self::Emph => p.push('_'),
            Self::Text(text) => p.push_str(text),
            Self::Raw(raw) => raw.pretty(p),
            Self::Math(math) => math.pretty(p),
            Self::Heading(heading) => heading.pretty(p),
            Self::List(list) => list.pretty(p),
            Self::Enum(enum_) => enum_.pretty(p),
            Self::Expr(expr) => {
                if expr.has_short_form() {
                    p.push('#');
                }
                expr.pretty(p);
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
            p.push_str(lang);
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

impl Pretty for MathNode {
    fn pretty(&self, p: &mut Printer) {
        p.push('$');
        if self.display {
            p.push('[');
        }
        p.push_str(&self.formula);
        if self.display {
            p.push(']');
        }
        p.push('$');
    }
}

impl Pretty for HeadingNode {
    fn pretty(&self, p: &mut Printer) {
        for _ in 0 .. self.level() {
            p.push('=');
        }
        p.push(' ');
        self.body().pretty(p);
    }
}

impl Pretty for ListNode {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("- ");
        self.body().pretty(p);
    }
}

impl Pretty for EnumNode {
    fn pretty(&self, p: &mut Printer) {
        if let Some(number) = self.number() {
            write!(p, "{}", number).unwrap();
        }
        p.push_str(". ");
        self.body().pretty(p);
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
            Self::With(v) => v.pretty(p),
            Self::Let(v) => v.pretty(p),
            Self::Set(v) => v.pretty(p),
            Self::Show(v) => v.pretty(p),
            Self::Wrap(v) => v.pretty(p),
            Self::If(v) => v.pretty(p),
            Self::While(v) => v.pretty(p),
            Self::For(v) => v.pretty(p),
            Self::Import(v) => v.pretty(p),
            Self::Include(v) => v.pretty(p),
        }
    }
}

impl Pretty for Lit {
    fn pretty(&self, p: &mut Printer) {
        match self.kind() {
            LitKind::None => p.push_str("none"),
            LitKind::Auto => p.push_str("auto"),
            LitKind::Bool(v) => write!(p, "{}", v).unwrap(),
            LitKind::Int(v) => write!(p, "{}", v).unwrap(),
            LitKind::Float(v) => write!(p, "{}", v).unwrap(),
            LitKind::Length(v, u) => write!(p, "{}{:?}", v, u).unwrap(),
            LitKind::Angle(v, u) => write!(p, "{}{:?}", v, u).unwrap(),
            LitKind::Percent(v) => write!(p, "{}%", v).unwrap(),
            LitKind::Fractional(v) => write!(p, "{}fr", v).unwrap(),
            LitKind::Str(v) => write!(p, "{:?}", v).unwrap(),
        }
    }
}

impl Pretty for ArrayExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');

        let items = self.items();
        let len = p.join(items, ", ", |item, p| item.pretty(p));
        if len == 1 {
            p.push(',');
        }
        p.push(')');
    }
}

impl Pretty for DictExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        let len = p.join(self.items(), ", ", |named, p| named.pretty(p));
        if len == 0 {
            p.push(':');
        }
        p.push(')');
    }
}

impl Pretty for Named {
    fn pretty(&self, p: &mut Printer) {
        self.name().pretty(p);
        p.push_str(": ");
        self.expr().pretty(p);
    }
}

impl Pretty for TemplateExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('[');
        self.body().pretty(p);
        p.push(']');
    }
}

impl Pretty for GroupExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('(');
        self.expr().pretty(p);
        p.push(')');
    }
}

impl Pretty for BlockExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push('{');
        if self.exprs().count() > 1 {
            p.push(' ');
        }
        let len = p.join(self.exprs(), "; ", |expr, p| expr.pretty(p));
        if len > 1 {
            p.push(' ');
        }
        p.push('}');
    }
}

impl Pretty for UnaryExpr {
    fn pretty(&self, p: &mut Printer) {
        let op = self.op();
        op.pretty(p);
        if op == UnOp::Not {
            p.push(' ');
        }
        self.expr().pretty(p);
    }
}

impl Pretty for UnOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

impl Pretty for BinaryExpr {
    fn pretty(&self, p: &mut Printer) {
        self.lhs().pretty(p);
        p.push(' ');
        self.op().pretty(p);
        p.push(' ');
        self.rhs().pretty(p);
    }
}

impl Pretty for BinOp {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self.as_str());
    }
}

impl Pretty for CallExpr {
    fn pretty(&self, p: &mut Printer) {
        self.callee().pretty(p);

        let mut write_args = |items: &[CallArg]| {
            p.push('(');
            p.join(items, ", ", |item, p| item.pretty(p));
            p.push(')');
        };

        let args: Vec<_> = self.args().items().collect();
        match args.as_slice() {
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
        p.join(self.items(), ", ", |item, p| item.pretty(p));
    }
}

impl Pretty for CallArg {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Pos(expr) => expr.pretty(p),
            Self::Named(named) => named.pretty(p),
            Self::Spread(expr) => {
                p.push_str("..");
                expr.pretty(p);
            }
        }
    }
}

impl Pretty for ClosureExpr {
    fn pretty(&self, p: &mut Printer) {
        let params: Vec<_> = self.params().collect();
        if let [param] = params.as_slice() {
            param.pretty(p);
        } else {
            p.push('(');
            p.join(params.iter(), ", ", |item, p| item.pretty(p));
            p.push(')');
        }
        p.push_str(" => ");
        self.body().pretty(p);
    }
}

impl Pretty for ClosureParam {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Pos(ident) => ident.pretty(p),
            Self::Named(named) => named.pretty(p),
            Self::Sink(ident) => {
                p.push_str("..");
                ident.pretty(p);
            }
        }
    }
}

impl Pretty for WithExpr {
    fn pretty(&self, p: &mut Printer) {
        self.callee().pretty(p);
        p.push_str(" with (");
        self.args().pretty(p);
        p.push(')');
    }
}

impl Pretty for LetExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("let ");
        self.binding().pretty(p);
        if let Some(Expr::Closure(closure)) = self.init() {
            p.push('(');
            p.join(closure.params(), ", ", |item, p| item.pretty(p));
            p.push_str(") = ");
            closure.body().pretty(p);
        } else if let Some(init) = self.init() {
            p.push_str(" = ");
            init.pretty(p);
        }
    }
}

impl Pretty for SetExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("set ");
        self.class().pretty(p);
        p.push_str("(");
        self.args().pretty(p);
        p.push(')');
    }
}

impl Pretty for ShowExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("show ");
        self.pattern().pretty(p);
        p.push_str(" as ");
        self.body().pretty(p);
    }
}

impl Pretty for WrapExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("wrap ");
        self.binding().pretty(p);
        p.push_str(" in ");
        self.body().pretty(p);
    }
}

impl Pretty for IfExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("if ");
        self.condition().pretty(p);
        p.push(' ');
        self.if_body().pretty(p);
        if let Some(expr) = self.else_body() {
            p.push_str(" else ");
            expr.pretty(p);
        }
    }
}

impl Pretty for WhileExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("while ");
        self.condition().pretty(p);
        p.push(' ');
        self.body().pretty(p);
    }
}

impl Pretty for ForExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("for ");
        self.pattern().pretty(p);
        p.push_str(" in ");
        self.iter().pretty(p);
        p.push(' ');
        self.body().pretty(p);
    }
}

impl Pretty for ForPattern {
    fn pretty(&self, p: &mut Printer) {
        if let Some(key) = self.key() {
            key.pretty(p);
            p.push_str(", ");
        }

        self.value().pretty(p);
    }
}

impl Pretty for ImportExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("import ");
        self.imports().pretty(p);
        p.push_str(" from ");
        self.path().pretty(p);
    }
}

impl Pretty for Imports {
    fn pretty(&self, p: &mut Printer) {
        match self {
            Self::Wildcard => p.push('*'),
            Self::Items(idents) => {
                p.join(idents, ", ", |item, p| item.pretty(p));
            }
        }
    }
}

impl Pretty for IncludeExpr {
    fn pretty(&self, p: &mut Printer) {
        p.push_str("include ");
        self.path().pretty(p);
    }
}

impl Pretty for Ident {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(self);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceFile;

    #[track_caller]
    fn roundtrip(src: &str) {
        test_parse(src, src);
    }

    #[track_caller]
    fn test_parse(src: &str, expected: &str) {
        let source = SourceFile::detached(src);
        let ast = source.ast().unwrap();
        let found = pretty(&ast);
        if found != expected {
            println!("tree:     {ast:#?}");
            println!("expected: {expected}");
            println!("found:    {found}");
            panic!("test failed");
        }
    }

    #[test]
    fn test_pretty_print_markup() {
        // Basic stuff.
        roundtrip("*");
        roundtrip("_");
        roundtrip(" ");
        roundtrip("\\ ");
        roundtrip("\n\n");
        roundtrip("hi");
        roundtrip("= *Ok*");
        roundtrip("- Ok");

        // Raw node.
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

        // Math node.
        roundtrip("$$");
        roundtrip("$a+b$");
        roundtrip("$[ a^2 + b^2 = c^2 ]$");
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
        roundtrip(r#"{"let's \" go"}"#);
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
        roundtrip("{x => x + 1}");
        roundtrip("{(a, b) => a + b}");

        // Control flow.
        roundtrip("#let x = 1 + 2");
        roundtrip("#let f(x) = y");
        roundtrip("#set text(size: 12pt)");
        roundtrip("#show heading(body) as [*{body}*]");
        roundtrip("#wrap body in columns(2, body)");
        roundtrip("#if x [y] else [z]");
        roundtrip("#if x {} else if y {} else {}");
        roundtrip("#while x {y}");
        roundtrip("#for x in y {z}");
        roundtrip("#for k, x in y {z}");
        roundtrip("#import * from \"file.typ\"");
        roundtrip("#include \"chapter1.typ\"");
    }
}
