//! Utility functionality.

use std::iter::Peekable;
use std::str::Split;
use unicode_xid::UnicodeXID;


/// Types that can be splined.
pub trait Splinor {
    /// Returns an iterator over the substrings splitted by the pattern,
    /// intertwined with the splinor.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[derive(Debug, Copy, Clone, PartialEq)]
    /// struct Space;
    ///
    /// let v: Vec<Splined<Space>> = "My airplane flies!".spline(" ", Space).collect();
    /// assert_eq!(v, [
    ///     Splined::Value("My"),
    ///     Splined::Splinor(Space),
    ///     Splined::Value("airplane"),
    ///     Splined::Splinor(Space),
    ///     Splined::Value("flies!"),
    /// ]);
    /// ```
    fn spline<'s, T: Clone>(&'s self, pat: &'s str, splinor: T) -> Spline<'s, T>;
}

impl Splinor for str {
    fn spline<'s, T: Clone>(&'s self, pat: &'s str, splinor: T) -> Spline<'s, T> {
        Spline {
            splinor: Splined::Splinor(splinor),
            split: self.split(pat).peekable(),
            next_splinor: false,
        }
    }
}

/// Iterator over splitted values and splinors.
///
/// Created by the [`spline`](Splinor::spline) function.
#[derive(Debug, Clone)]
pub struct Spline<'s, T> {
    splinor: Splined<'s, T>,
    split: Peekable<Split<'s, &'s str>>,
    next_splinor: bool,
}

/// Represents either a splitted substring or a splinor.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Splined<'s, T> {
    /// A substring.
    Value(&'s str),
    /// An intertwined splinor.
    Splinor(T),
}

impl<'s, T: Clone> Iterator for Spline<'s, T> {
    type Item = Splined<'s, T>;

    fn next(&mut self) -> Option<Splined<'s, T>> {
        if self.next_splinor && self.split.peek().is_some() {
            self.next_splinor = false;
            return Some(self.splinor.clone());
        } else {
            self.next_splinor = true;
            return Some(Splined::Value(self.split.next()?))
        }
    }
}

/// More useful functions on `str`'s.
pub trait StrExt {
    /// Whether self consists only of whitespace.
    fn is_whitespace(&self) -> bool;

    /// Whether this word is a valid unicode identifier.
    fn is_identifier(&self) -> bool;
}

impl StrExt for str {
    fn is_whitespace(&self) -> bool {
        self.chars().all(|c| c.is_whitespace() && c != '\n')
    }

    fn is_identifier(&self) -> bool {
        let mut chars = self.chars();

        match chars.next() {
            Some(c) if !UnicodeXID::is_xid_start(c) => return false,
            None => return false,
            _ => (),
        }

        while let Some(c) = chars.next() {
            if !UnicodeXID::is_xid_continue(c) {
                return false;
            }
        }

        true
    }
}


#[cfg(test)]
mod splinor_tests {
    use super::*;
    use Splined::{Value as V, Splinor as S};

    #[derive(Debug, Copy, Clone, PartialEq)]
    enum Token { DoubleUnderscore }

    fn test<T>(string: &str, pat: &str, splinor: T, vec: Vec<Splined<T>>)
        where T: std::fmt::Debug + Clone + PartialEq {
        assert_eq!(string.spline(pat, splinor).collect::<Vec<_>>(), vec);
    }

    #[test]
    fn splinor() {
        let s = S(Token::DoubleUnderscore);
        test("__he__llo__world__", "__", Token::DoubleUnderscore,
             vec![V(""), s, V("he"), s, V("llo"), s, V("world"), s, V("")]);
        test("__Italic__", "__", Token::DoubleUnderscore,
             vec![V(""), s, V("Italic"), s, V("")]);
        test("Key__Value", "__", Token::DoubleUnderscore,
             vec![V("Key"), s, V("Value")]);
        test("__Start__NoEnd", "__", Token::DoubleUnderscore,
             vec![V(""), s, V("Start"), s, V("NoEnd")]);
        test("NoStart__End__", "__", Token::DoubleUnderscore,
             vec![V("NoStart"), s, V("End"), s, V("")]);
    }
}
