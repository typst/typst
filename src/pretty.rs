//! Pretty printing.

use std::fmt::{Arguments, Result, Write};

use crate::color::{Color, RgbaColor};
use crate::geom::{Angle, Length, Linear, Relative};

/// Pretty print an item and return the resulting string.
pub fn pretty<T>(item: &T) -> String
where
    T: Pretty + ?Sized,
{
    let mut p = Printer::new();
    item.pretty(&mut p);
    p.finish()
}

/// Pretty printing.
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
    pub fn write_fmt(&mut self, fmt: Arguments<'_>) -> Result {
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
    fn write_str(&mut self, s: &str) -> Result {
        self.push_str(s);
        Ok(())
    }
}

impl Pretty for i64 {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(itoa::Buffer::new().format(*self));
    }
}

impl Pretty for f64 {
    fn pretty(&self, p: &mut Printer) {
        p.push_str(ryu::Buffer::new().format(*self));
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

macro_rules! impl_pretty_display {
    ($($type:ty),* $(,)?) => {
        $(impl Pretty for $type {
            fn pretty(&self, p: &mut Printer) {
                write!(p, "{}", self).unwrap();
            }
        })*
    };
}

impl_pretty_display! {
    bool,
    Length,
    Angle,
    Relative,
    Linear,
    RgbaColor,
    Color,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pretty_print_str() {
        assert_eq!(pretty("\n"), r#""\n""#);
        assert_eq!(pretty("\\"), r#""\\""#);
        assert_eq!(pretty("\""), r#""\"""#);
    }
}
