// use std::borrow::Cow;

// use ecow::EcoString;

// use crate::diag::{At, SourceResult};
use crate::engine::Engine;
use crate::foundations::func;
// use crate::loading::{Encoding, Readable};
// use crate::syntax::Spanned;
use crate::World;

/// Returns absolute path to the  main file that represents document's entry
/// point. Returns `{none}` if `stdin` is used instead.
#[func]
pub fn entrypoint(
    /// The engine.
    engine: &mut Engine,
    // /// The encoding to read the file with.
    // ///
    // /// If set to `{none}`, this function returns raw bytes.
    // #[named]
    // #[default(Some(Encoding::Utf8))]
    // encoding: Option<Encoding>,
    // ) -> SourceResult<Readable> {
) -> Option<String> {
    // let Spanned { v: path, span } = path;
    let file_id = engine.world.main().id();
    if engine.world.is_stdin(file_id) {
        return None;
    }
    dbg!(&file_id);
    let rootless_path = file_id.vpath().as_rootless_path();
    dbg!(rootless_path);
    let rooted_path = file_id.vpath().as_rooted_path();
    dbg!(rooted_path);
    let string = rooted_path.to_string_lossy();
    Some(string.into())
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

// /// An encoding of a file.
// #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
// pub enum Encoding {
//     /// The Unicode UTF-8 encoding.
//     Utf8,
// }
