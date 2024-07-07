// use std::borrow::Cow;

// use ecow::EcoString;

use crate::diag::SourceResult;
use crate::engine::Engine;
use crate::foundations::{func, Bytes, Str};
use typst_macros::cast;
// use crate::loading::{Encoding, Readable};
// use crate::syntax::Spanned;
use crate::World;

/// A value that can be read from a file.
#[derive(Debug, Clone, PartialEq, Hash)]
pub enum Entrypoint {
    /// Lossy string.
    Str(Str),
    /// Raw bytes.
    Bytes(Bytes),
}

/// Returns absolute path to the  main file that represents document's entry
/// point. Returns `{none}` if `stdin` is used instead.
#[func]
pub fn entrypoint(
    /// The engine.
    engine: &mut Engine,
    /// Whether to return `{bytes}` or not.
    ///
    /// If set to `{true}`, this function returns raw bytes. Default is
    /// `{false}`. Useful when path cannot be a valid UTF-8 string.
    #[named]
    #[default(false)]
    raw: bool,
) -> SourceResult<Option<Entrypoint>> {
    // let Spanned { v: path, span } = path;
    let file_id = engine.world.main().id();
    if engine.world.main().is_stdin() {
        return Ok(None);
    }
    dbg!(&file_id);
    let rootless_path = file_id.vpath().as_rootless_path();
    dbg!(rootless_path);
    let rooted_path = file_id.vpath().as_rooted_path();
    dbg!(rooted_path);
    if raw {
        let bytes: Vec<u8> = rooted_path.as_os_str().as_encoded_bytes().to_vec();
        return Ok(Some(Entrypoint::Bytes(bytes.into())));
    }
    let string = rooted_path.to_string_lossy();
    Ok(Some(Entrypoint::Str(string.into())))
    // let data = engine.world.file(id).at(span)?;
    // Ok(match encoding {
    //     None => Readable::Bytes(data),
    //     Some(Encoding::Utf8) => Readable::Str(
    //         std::str::from_utf8(&data)
    //             .map_err(|_| "file is not valid utf-8")
    //             .at(span)?
    //             .into(),
    //     ),
    // })
}

cast! {
    Entrypoint,
    self => match self {
        Self::Str(v) => v.into_value(),
        Self::Bytes(v) => v.into_value(),
    },
    v: Str => Self::Str(v),
    v: Bytes => Self::Bytes(v),
}

// /// An encoding of a file.
// #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
// pub enum Encoding {
//     /// The Unicode UTF-8 encoding.
//     Utf8,
// }
