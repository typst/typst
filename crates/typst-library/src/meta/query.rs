use crate::prelude::*;

/// Finds elements in the document.
///
/// The `query` functions lets you search your document for elements of a
/// particular type or with a particular label. To use it, you first need to
/// retrieve the current document location with the [`locate`]($locate)
/// function.
///
/// # Finding elements
/// In the example below, we create a custom page header that displays the text
/// "Typst Academy" in small capitals and the current section title. On the
/// first page, the section title is omitted because the header is before the
/// first section heading.
///
/// To realize this layout, we call `locate` and then query for all headings
/// after the current location. The function we pass to locate is called twice
/// in this case: Once per page.
///
/// - On the first page the query for all headings before the current location
///   yields an empty array: There are no previous headings. We check for this
///   case and and just display "Typst Academy".
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
/// #set page(header: locate(loc => {
///   let elems = query(
///     selector(heading).before(loc),
///     loc,
///   )
///   let academy = smallcaps[
///     Typst Academy
///   ]
///   if elems == () {
///     align(right, academy)
///   } else {
///     let body = elems.last().body
///     academy + h(1fr) + emph(body)
///   }
/// }))
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
/// on. As we can see, the output has five headings. This is because Typst
/// simply gives up after five attempts.
///
/// In general, you should try not to write queries that affect themselves. The
/// same words of caution also apply to other introspection features like
/// [counters]($counter) and [state]($state).
///
/// ```example
/// = Real
/// #locate(loc => {
///   let elems = query(heading, loc)
///   let count = elems.len()
///   count * [= Fake]
/// })
/// ```
///
/// # Command line queries
/// You can also perform queries from the command line with the `typst query`
/// command. This command executes an arbitrary query on the document and
/// returns the resulting elements in serialized form. Consider the following
/// `example.typ` file which contains some invisible [metadata]($metadata):
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
#[func]
pub fn query(
    /// The virtual machine.
    vm: &mut Vm,
    /// Can be an element function like a `heading` or `figure`, a `{<label>}`
    /// or a more complex selector like `{heading.where(level: 1)}`.
    ///
    /// Currently, only a subset of element functions is supported. Aside from
    /// headings and figures, this includes equations, references and all
    /// elements with an explicit label. As a result, you _can_ query for e.g.
    /// [`strong`]($strong) elements, but you will find only those that have an
    /// explicit label attached to them. This limitation will be resolved in the
    /// future.
    target: LocatableSelector,
    /// Can be an arbitrary location, as its value is irrelevant for the
    /// function's return value. Why is it required then? As noted before, Typst
    /// has to evaluate parts of your code multiple times to determine the
    /// values of all state. By only allowing this function within
    /// [`locate`]($locate) calls, the amount of code that can depend on the
    /// query's result is reduced. If you could call it directly at the top
    /// level of a module, the evaluation of the whole module and its exports
    /// could depend on the query's result.
    location: Location,
) -> Array {
    let _ = location;
    let vec = vm.vt.introspector.query(&target.0);
    vec.into_iter()
        .map(|elem| Value::Content(elem.into_inner()))
        .collect()
}
