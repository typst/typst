use std::fmt::Write;

use typst::foundations::Smart;
use typst::model::{Document, DocumentInfo};
use typst::World;

use crate::collect::Test;
use crate::world::TestWorld;

/// We don't want to panic when there is a failure.
macro_rules! test_eq {
    ($sink:expr, $lhs:expr, $rhs:expr) => {
        if $lhs != $rhs {
            writeln!(&mut $sink, "{:?} != {:?}", $lhs, $rhs).unwrap();
        }
    };
}

/// Run special checks for specific tests for which it is not worth it to create
/// custom annotations.
pub fn check(test: &Test, world: &TestWorld, doc: Option<&Document>) -> String {
    let mut sink = String::new();
    match test.name.as_str() {
        "document-set-author-date" => {
            let info = info(doc);
            test_eq!(sink, info.author, ["A", "B"]);
            test_eq!(sink, info.date, Smart::Custom(world.today(None)));
        }
        "issue-4065-document-context" => {
            let info = info(doc);
            test_eq!(sink, info.title.as_deref(), Some("Top level"));
        }
        "issue-4769-document-context-conditional" => {
            let info = info(doc);
            test_eq!(sink, info.author, ["Changed"]);
            test_eq!(sink, info.title.as_deref(), Some("Alternative"));
        }
        _ => {}
    }
    sink
}

/// Extract the document information.
fn info(doc: Option<&Document>) -> DocumentInfo {
    doc.map(|doc| doc.info.clone()).unwrap_or_default()
}
