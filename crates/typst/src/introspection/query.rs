use comemo::Tracked;

use crate::diag::HintedStrResult;
use crate::engine::Engine;
use crate::foundations::{func, Array, Context, LocatableSelector, Value};
use crate::introspection::Location;

/// Finds elements in the document.
///
/// The `query` functions lets you search your document for elements of a
/// particular type or with a particular label. To use it, you first need to
/// ensure that [context] is available.
///

/// # Finding elements
/// In the example below, we create a custom page header that displays the text
/// "Typst Academy" in small capitals and the current section title. On the
/// first page, the section title is omitted because the header is before the
/// first section heading.
///
/// To realize this layout, we open a `context` and then query for all headings
/// after the [current location]($here). The code within the context block
/// runs twice: Once per page.
///
/// - On the first page the query for all headings before the current location
///   yields an empty array: There are no previous headings. We check for this
///   case and just display "Typst Academy".
///
/// - For the second page, we retrieve the last element from the query's result.
///   This is the latest heading before the current position and as such, it is
///   the heading of the section we are currently in. We access its content
///   through the `body` field and display it alongside "Typst Academy".
///
/// ```example
/// >>> #set page(
/// >>>   width: 240pt,
/// >>>   height: 180pt,
/// >>>   margin: (top: 35pt, rest: 15pt),
/// >>>   header-ascent: 12pt,
/// >>> )
/// #set page(header: context {
///   let elems = query(
///     selector(heading).before(here()),
///   )
///   let academy = smallcaps[
///     Typst Academy
///   ]
///   if elems.len() == 0 {
///     align(right, academy)
///   } else {
///     let body = elems.last().body
///     academy + h(1fr) + emph(body)
///   }
/// })
///
/// = Introduction
/// #lorem(23)
///
/// = Background
/// #lorem(30)
///
/// = Analysis
/// #lorem(15)
/// ```
///
/// You can get the location of the elements returned by `query` with
/// [`location`]($content.location).
///
/// # A word of caution { #caution }
/// To resolve all your queries, Typst evaluates and layouts parts of the
/// document multiple times. However, there is no guarantee that your queries
/// can actually be completely resolved. If you aren't careful a query can
/// affect itself—leading to a result that never stabilizes.
///
/// In the example below, we query for all headings in the document. We then
/// generate as many headings. In the beginning, there's just one heading,
/// titled `Real`. Thus, `count` is `1` and one `Fake` heading is generated.
/// Typst sees that the query's result has changed and processes it again. This
/// time, `count` is `2` and two `Fake` headings are generated. This goes on and
/// on. As we can see, the output has a finite amount of headings. This is
/// because Typst simply gives up after a few attempts.
///
/// In general, you should try not to write queries that affect themselves. The
/// same words of caution also apply to other introspection features like
/// [counters]($counter) and [state].
///
/// ```example
/// = Real
/// #context {
///   let elems = query(heading)
///   let count = elems.len()
///   count * [= Fake]
/// }
/// ```
///
/// # Command line queries
/// You can also perform queries from the command line with the `typst query`
/// command. This command executes an arbitrary query on the document and
/// returns the resulting elements in serialized form. Consider the following
/// `example.typ` file which contains some invisible [metadata]:
///
/// ```typ
/// #metadata("This is a note") <note>
/// ```
///
/// You can execute a query on it as follows using Typst's CLI:
/// ```sh
/// $ typst query example.typ "<note>"
/// [
///   {
///     "func": "metadata",
///     "value": "This is a note",
///     "label": "<note>"
///   }
/// ]
/// ```
///
/// Frequently, you're interested in only one specific field of the resulting
/// elements. In the case of the `metadata` element, the `value` field is the
/// interesting one. You can extract just this field with the `--field`
/// argument.
///
/// ```sh
/// $ typst query example.typ "<note>" --field value
/// ["This is a note"]
/// ```
///
/// If you are interested in just a single element, you can use the `--one`
/// flag to extract just it.
///
/// ```sh
/// $ typst query example.typ "<note>" --field value --one
/// "This is a note"
/// ```
#[func(contextual)]
pub fn query(
    /// The engine.
    engine: &mut Engine,
    /// The callsite context.
    context: Tracked<Context>,
    /// Can be
    /// - an element function like a `heading` or `figure`,
    /// - a `{<label>}`,
    /// - a more complex selector like `{heading.where(level: 1)}`,
    /// - or `{selector(heading).before(here())}`.
    ///
    /// Only [locatable]($location/#locatable) element functions are supported.
    target: LocatableSelector,
    /// _Compatibility:_ This argument only exists for compatibility with
    /// Typst 0.10 and lower and shouldn't be used anymore.
    #[default]
    location: Option<Location>,
) -> HintedStrResult<Array> {
    if location.is_none() {
        context.introspect()?;
    }

    let vec = engine.introspector.query(&target.0);
    Ok(vec.into_iter().map(Value::Content).collect())
}
