//! borrow from <https://github.com/netvl/xml-rs/blob/277656386e9910a395e2232e9f6e21d1da0e06c2/src/escape.rs>

use core::fmt;
use std::{borrow::Cow, marker::PhantomData};

pub(crate) trait Escapes {
    fn escape(c: u8) -> Option<&'static str>;

    fn byte_needs_escaping(c: u8) -> bool {
        Self::escape(c).is_some()
    }

    fn str_needs_escaping(s: &str) -> bool {
        s.bytes().any(|c| Self::escape(c).is_some())
    }
}

pub(crate) struct Escaped<'a, E: Escapes> {
    _escape_phantom: PhantomData<E>,
    to_escape: &'a str,
}

impl<'a, E: Escapes> Escaped<'a, E> {
    pub fn new(s: &'a str) -> Self {
        Escaped { _escape_phantom: PhantomData, to_escape: s }
    }
}

impl<'a, E: Escapes> fmt::Display for Escaped<'a, E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut total_remaining = self.to_escape;

        // find the next occurence
        while let Some(n) = total_remaining.bytes().position(E::byte_needs_escaping) {
            let (start, remaining) = total_remaining.split_at(n);

            f.write_str(start)?;

            // unwrap is safe because we checked is_some for position n earlier
            let next_byte = remaining.bytes().next().unwrap();
            let replacement = E::escape(next_byte).unwrap();
            f.write_str(replacement)?;

            total_remaining = &remaining[1..];
        }

        f.write_str(total_remaining)
    }
}

pub(crate) fn escape_str<E: Escapes>(s: &str) -> Cow<'_, str> {
    if E::str_needs_escaping(s) {
        Cow::Owned(format!("{}", Escaped::<E>::new(s)))
    } else {
        Cow::Borrowed(s)
    }
}

macro_rules! escapes {
    {
        $name: ident,
        $($k: expr => $v: expr),* $(,)?
    } => {
        pub(crate) struct $name;

        impl Escapes for $name {
            fn escape(c: u8) -> Option<&'static str> {
                match c {
                    $( $k => Some($v),)*
                    _ => None
                }
            }
        }
    };
}

escapes!(
    TextContentDataEscapes,
    b'<' => "&lt;",
    b'&' => "&amp;",
    // also excaple space
    b' ' => "&nbsp;",
);
