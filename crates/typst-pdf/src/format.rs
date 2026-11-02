//! PDF-specific functionality.

use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::num::NonZeroU32;

use ecow::{EcoString, eco_format};
use krilla::configure::{Accessibility, Archival, PdfVersion, Validator};
use serde::{Deserialize, Serialize};
use typst_library::World;
use typst_library::diag::{
    At, HintedStrResult, HintedString, SourceResult, StrResult, bail,
};
use typst_library::engine::Engine;
use typst_library::format::{Complete, Fields, Format, FormatElement, Partial, Populate};
use typst_library::foundations::{
    Args, Array, Bytes, Construct, Content, Derived, IntoValue, NativeElement,
    NativeRuleMap, PathOrStr, ShowFn, Smart, StyleChain, Target, Value,
};
use typst_library::layout::PageRanges;
use typst_library::model::{
    ArtifactElem, TableCell, TableCellKind, TableElem, TableHeaderScope,
};
use typst_macros::{Cast, cast, elem, func, scope};
use typst_syntax::Spanned;
use typst_utils::NonZeroExt;

/// The format element for registering the PDF format.
pub const FORMAT: Format = Format::new::<PdfFormat>().with_rules(register);

/// Registers show rules for PDF specific elements.
pub fn register(rules: &mut NativeRuleMap) {
    rules.register(Target::Paged, ATTACH_RULE);
}

const ATTACH_RULE: ShowFn<AttachElem> = |_, _, _| Ok(Content::empty());

/// Typst's PDF export format.
///
/// PDF files focus on accurately describing documents visually, but also have
/// facilities for annotating their structure. This hybrid approach makes them a
/// good fit for document exchange: They render exactly the same on every
/// device, but also support extraction of a document's content and structure
/// (at least to an extent). Unlike PNG files, PDFs are not bound to a specific
/// resolution. Hence, you can view them at any size without incurring a loss of
/// quality.
///
/// = Exporting as PDF <exporting-as-pdf>
/// == Command Line <command-line>
/// PDF is Typst's default export format. Running the `compile` or `watch`
/// subcommand without specifying a format will create a PDF. When exporting to
/// PDF, you have the following configuration options:
///
/// - Which @pdf.standard[PDF standards] Typst should enforce conformance with
///   by specifying `--pdf-standard` followed by one or multiple comma-separated
///   standards. Valid standards are `1.4`, `1.5`, `1.6`, `1.7`, `2.0`, `a-1b`,
///   `a-1a`, `a-2b`, `a-2u`, `a-2a`, `a-3b`, `a-3u`, `a-3a`, `a-4`, `a-4f`,
///   `a-4e`, and `ua-1`. By default, Typst outputs PDF-1.7-compliant files.
///
/// - You can disable PDF tagging completely with `--pdf-tagged=false`. By
///   default, Typst will always write _Tagged PDF_ to provide a baseline level
///   of accessibility. Using this flag, you can turn tags off. This will make
///   your file inaccessible and prevent conformance with accessible conformance
///   levels of PDF/A and all parts of PDF/UA.
///
/// - Which pages to export by specifying `--pages` followed by a
///   comma-separated list of numbers or dash-separated number ranges. Ranges
///   can be half-open. Example: `2,3,7-9,11-`.
///
/// == Web App <pdf:web-app>
/// Click the quick download button at the top right to export a PDF with
/// default settings. For further configuration, click "File" > "Export as" >
/// "PDF" or click the downwards-facing arrow next to the quick download button
/// and select "Export as PDF". When exporting to PDF, you have the following
/// configuration options:
///
/// - Which PDF standards Typst should enforce conformance with. By default,
///   Typst outputs PDF-1.7-compliant files. You can choose the PDF version
///   freely between 1.4 and 2.0. Valid additional standards are `A-1b`, `A-1a`,
///   `A-2b`, `A-2u`, `A-2a`, `A-3b`. `A-3u`, `A-3a`, `A-4`, `A-4f`, `A-4e`, and
///   `UA-1`.
///
/// - Which pages to export. Valid options are "All pages", "Current page", and
///   "Custom ranges". Custom ranges are a comma-separated list of numbers or
///   dash-separated number ranges. Ranges can be half-open. Example:
///   `2,3,7-9,11-`.
///
/// = PDF-specific functionality <pdf-specific-functionality>
/// Typst exposes PDF-specific functionality in the global `pdf` element. See
/// below for the definitions and options it contains.
///
/// This element contains some functions without a final API. They are designed
/// to enhance accessibility for documents with complex tables. This includes
/// @[`table-summary`], @pdf.header-cell[`header-cell`], and
/// @pdf.data-cell[`data-cell`]. All of these functions will be removed in a
/// future Typst release, either through integration into table functions or
/// through full removal. You can enable these functions by passing `--features
/// a11y-extras` or setting the `TYPST_FEATURES` environment variable to
/// `a11y-extras`. In the web app, these features are not available at this
/// time.
#[elem(scope, name = "pdf", title = "PDF", since = "0.13.0", Construct)]
pub struct PdfFormat {
    /// Specifies which ranges of pages should be included in the PDF. This can
    /// be:
    ///
    /// - `{none}` to export all pages.
    /// - A single page range.
    /// - An array of page ranges.
    ///
    /// A page range is one of:
    ///
    /// - A number that specifies exactly one page (page numbers start at one).
    /// - A dictionary with the keys `from` (indicating the start of a page
    ///   range) and/or `to` (indicating the end of a page range). At least one
    ///   of the keys must be present, and both the start and end numbers are
    ///   inclusive. Thus the range `{(from: 1, to: 3)}`, is the list of pages
    ///   `{(1, 2, 3)}`.
    ///
    /// Specifying a single page or page range:
    /// ```typ
    /// #set pdf(pages: 3)
    /// #set pdf(pages: (from: 3))
    /// #set pdf(pages: (to: 5))
    /// #set pdf(pages: (from: 3, to: 5))
    /// ```
    ///
    /// Specifying multiple pages, page ranges, or a mix:
    /// ```typ
    /// #set pdf(pages: (5, 7, 9))
    /// #set pdf(pages: ((to: 3), 5, (from: 7, to: 8), (from: 10)))
    /// ```
    #[default]
    pub pages: Option<PageRanges>,

    /// A list of PDF standards that Typst will enforce conformance with. This
    /// can be a single standard or an array of standards.
    ///
    /// The International Standards Organization (ISO) has published the base
    /// PDF standard and various standards that extend it to make PDFs more
    /// suitable for specific use-cases. By default, Typst exports PDF 1.7
    /// files. Adobe Acrobat 8 and later as well as all other commonly used PDF
    /// viewers are compatible with this PDF version.
    ///
    /// Some features of Typst may not be available depending on the PDF
    /// standard you choose. You can combine compatible PDF/A and PDF/UA
    /// standards. Notably, PDF/UA-1 and PDF/A-4 are mutually incompatible since
    /// the former requires PDF 1.7 or earlier and the latter requires PDF 2.0.
    ///
    /// = PDF versions <pdf-versions>
    /// Typst supports five different PDF versions: 1.4, 1.5, 1.6, 1.7
    /// (default), and 2.0. You can choose each of these versions for your
    /// document export. However, based on the features you used there may be a
    /// minimum version. Likewise, the standards you target can limit which
    /// versions you can choose (see below for more).
    ///
    /// Here is a list on how each new version improves over PDF 1.4 for Typst
    /// documents:
    ///
    /// - *PDF 1.5* (2003): Improved color management, text extraction, table
    ///   accessibility, reflow, and emoji fonts
    /// - *PDF 1.6* (2004): More flexible links
    /// - *PDF 1.7* (2006): Allows @pdf.attach[attachments], improved reflow
    /// - *PDF 2.0* (2017): Improved both metadata and tag semantics for
    ///   accessibility
    ///
    /// The software used to read your file must support your PDF version. Under
    /// normal circumstances, this poses no problem, but it can be a source of
    /// errors when working with older hardware. For general exchange, we
    /// recommend keeping the default PDF 1.7 setting or choosing PDF 2.0.
    ///
    /// When using PDF files as @image[images] in your document, the export PDF
    /// version must equal or exceed the image file versions.
    ///
    /// = PDF/UA <pdf-ua>
    /// Typst supports writing PDF/UA-conformant files. PDF/UA files are
    /// designed for _@guides:accessibility:basics[Universal Access]._ When you
    /// choose this PDF standard, Typst will run additional checks when
    /// exporting your document. These checks will make sure that you are
    /// following accessibility best practices. For example, it will make sure
    /// that all your images come with alternative descriptions.
    ///
    /// Note that there are some rules in PDF/UA that are crucial for
    /// accessibility but cannot be automatically checked. Hence, when exporting
    /// a PDF/UA-1 document, make sure you did the following:
    ///
    /// - If your document is written in a different language than English, make
    ///   sure @text.lang[set the text language] before any content.
    /// - Make sure to use Typst's semantic elements (like @heading[headings],
    ///   @figure[figures], and @list[lists]) when appropriate instead of
    ///   defining custom constructs. This lets Typst know (and export) the role
    ///   a construct plays in the document. See @guides:accessibility[the
    ///   Accessibility guide] for more details.
    /// - Do not exclusively use contrast, color, format, or layout to
    ///   communicate an idea. Use text or alternative descriptions instead of
    ///   or in addition to these elements.
    /// - Wrap all decorative elements without a semantic meaning in
    ///   @pdf.artifact.
    /// - Do not use images of text. Instead, insert the text directly into your
    ///   markup.
    ///
    /// Typst currently only supports part one (PDF/UA-1) which is based on PDF
    /// 1.7 (2006). When exporting to PDF/UA-1, be aware that you will need to
    /// manually provide @math:accessibility[alternative descriptions of
    /// mathematics] in natural language.
    ///
    /// New accessibility features were added to PDF 2.0 (2017). When set to PDF
    /// 2.0 export, Typst will leverage some of these features. PDF 2.0 and
    /// PDF/UA-1, however, are mutually incompatible. For accessible documents,
    /// we currently recommend exporting to PDF/UA-1 instead of PDF 2.0 for the
    /// additional checks and greater compatibility. The second part of PDF/UA
    /// is designed for PDF 2.0, but not yet supported by Typst.
    ///
    /// = PDF/A <pdf-a>
    /// Typst optionally supports emitting PDF/A-conformant files. PDF/A files
    /// are geared towards maximum compatibility with current and future PDF
    /// tooling. They do not rely on difficult-to-implement or proprietary
    /// features and contain exhaustive metadata. This makes them suitable for
    /// long-term archival.
    ///
    /// The PDF/A Standard has multiple versions (_parts_ in ISO terminology)
    /// and most parts have multiple profiles that indicate the file's
    /// conformance level. You can target one part and conformance level at a
    /// time. Currently, Typst supports these PDF/A output profiles:
    ///
    /// - *PDF/A-1b:* The _basic_ conformance level of ISO 19005-1. This version
    ///   of PDF/A is based on PDF 1.4 (2001) and results in self-contained,
    ///   archivable PDF files. As opposed to later parts of the PDF/A standard,
    ///   transparency is not allowed in PDF/A-1 files.
    ///
    /// - *PDF/A-1a:* This is the _accessible_ conformance level that builds on
    ///   the basic level PDF/A-1b. To conform to this level, your file must be
    ///   an accessible _Tagged PDF_ file. Note that accessibility is improved
    ///   with later parts as not all PDF accessibility features are available
    ///   for PDF 1.4 files. Furthermore, all text in the file must consist of
    ///   known Unicode code points.
    ///
    /// - *PDF/A-2b:* The _basic_ conformance level of ISO 19005-2. This version
    ///   of PDF/A is based on PDF 1.7 (2006) and results in self-contained,
    ///   archivable PDF files.
    ///
    /// - *PDF/A-2u:* This is the _Unicode-mappable_ conformance level that
    ///   builds on the basic level A-2b. It also adds rules that all text in
    ///   the document must consist of known Unicode code points. If possible,
    ///   always prefer this standard over PDF/A-2b.
    ///
    /// - *PDF/A-2a:* This is the _accessible_ conformance level that builds on
    ///   the Unicode-mappable level A-2u. This conformance level also adds two
    ///   requirements: Your file must be an accessible _Tagged PDF_ file. Typst
    ///   automatically adds tags to help you reach this conformance level. Also
    ///   pay attention to the Accessibility Sections throughout the reference
    ///   and the @guides:accessibility[Accessibility Guide] when targeting this
    ///   conformance level. Finally, PDF/A-2a forbids you from using code
    ///   points in the
    ///   #link("https://en.wikipedia.org/wiki/Private_Use_Areas")[Unicode
    ///   Private Use area]. If you want to build an accessible file, also
    ///   consider additionally targeting PDF/UA-1, which enables more automatic
    ///   accessibility checks.
    ///
    /// - *PDF/A-3b:* The _basic_ conformance level of ISO 19005-3. This version
    ///   of PDF/A is based on PDF 1.7 (2006) and results in archivable PDF
    ///   files that can contain arbitrary other related files as
    ///   @pdf.attach[attachments]. The only difference between it and PDF/A-2b
    ///   is the capability to attach non-PDF/A-conformant files.
    ///
    /// - *PDF/A-3u:* This is the _Unicode-mappable_ conformance level that
    ///   builds on the basic level A-3b. Just like PDF/A-2b, this requires all
    ///   text to consist of known Unicode code points. These rules do not apply
    ///   to attachments. If possible, always prefer this standard over
    ///   PDF/A-3b.
    ///
    /// - *PDF/A-3a:* This is the _accessible_ conformance level that builds on
    ///   the Unicode-mappable level A-3u. Just like PDF/A-2a, this requires
    ///   files to be accessible _Tagged PDF_ and to not use characters from the
    ///   Unicode Private Use area. Just like before, these rules do not apply
    ///   to attachments.
    ///
    /// - *PDF/A-4:* The basic conformance level of ISO 19005-4. This version of
    ///   PDF/A is based on PDF 2.0 (2017) and results in self-contained,
    ///   archivable PDF files. PDF/A-4 has no parts relating to accessibility.
    ///   Instead, the topic has been elaborated on more in the dedicated PDF/UA
    ///   standard. PDF/A-4 files can conform to PDF/UA-2 (currently not
    ///   supported in Typst).
    ///
    /// - *PDF/A-4f:* The _embedded files_ conformance level that builds on the
    ///   basic level A-4. Files conforming to this level can contain arbitrary
    ///   other related files as @pdf.attach[attachments], just as files
    ///   conforming to part 3 of ISO 19005. The only difference between it and
    ///   PDF/A-4 is the capability to attach non-PDF/A-conformant files.
    ///
    /// - *PDF/A-4e:* The _engineering_ conformance level that builds on the
    ///   embedded files level A-4f. Files conforming to this level can contain
    ///   3D objects. Typst does not support 3D content, so this is functionally
    ///   equivalent to PDF/A-4f from a Typst perspective.
    ///
    /// If you want to target PDF/A but are unsure about which particular
    /// setting to use, there are some good rules of thumb. First, you must
    /// determine your *part,* that's the "version number" of the standard. Ask
    /// yourself these questions:
    ///
    /// + *Does pre-2006 software or equipment need to be able to read my file?*
    ///   If so, choose part one (PDF/A-1).
    ///
    /// + *If not, does my file need @pdf.attach[attachments]?* If so, you must
    ///   choose part three (PDF/A-3) or the embedded files level of part four
    ///   (PDF/A-4f).
    ///
    /// + *Does your file need to use features introduced in PDF 2.0?*
    ///   Currently, use of PDF 2.0 features in Typst is limited to minor
    ///   improvements in accessibility, e.g. for the @title[title element]. If
    ///   you can do without these improvements, maximize compatibility by
    ///   choosing part two (when you don't need attachments) or part three
    ///   (when you do need attachments). If you rely on PDF 2.0 features, use
    ///   part four.
    ///
    /// Now, you only need to choose a *conformance level* (the lowercase letter
    /// at the end).
    ///
    /// - *If you decided on part one, two, or three,* you should typically
    ///   choose the accessible conformance level of your part, indicated by a
    ///   lowercase `a` at the end. If your document is inherently inaccessible,
    ///   e.g. an artist's portfolio that cannot be boiled down to alternative
    ///   descriptions, choose conformance level `u` instead. Only if this
    ///   results in a compiler error, e.g. because you used code points from
    ///   the Unicode Private Use Area, use the basic level `b`.
    ///
    /// - *If you have chosen part four,* you should choose the basic
    ///   conformance level (PDF/A-4) except when needing to embed files.
    ///
    /// When choosing between exporting PDF/A and regular PDF, keep in mind that
    /// PDF/A files contain additional metadata, and that some readers will
    /// prevent the user from modifying a PDF/A file.
    #[default]
    pub standard: PdfStandards,

    /// Whether to produce a tagged PDF document.
    ///
    /// Tagging is enabled by default to provide a baseline of accessibility. It
    /// can be turned off manually, e.g. to reduce the size of the document, and
    /// will be disabled automatically when exporting a specific page range.
    #[default]
    pub tagged: Smart<bool>,

    /// Whether to pretty-print the produced PDF document.
    ///
    /// This formats the output in a more human-readable, but less
    /// space-efficient way.
    #[default(false)]
    pub pretty: bool,
}

impl FormatElement for PdfFormat {
    type Options = PdfFormatOptions;
}

impl Construct for PdfFormat {
    fn construct(_: &mut Engine, args: &mut Args) -> SourceResult<Content> {
        bail!(args.span, "cannot be constructed manually")
    }
}

#[scope(category = Pdf)]
impl PdfFormat {
    #[elem]
    type AttachElem;

    #[elem]
    type ArtifactElem;

    /// A summary of the purpose and structure of a complex table.
    ///
    /// This will be available for Assistive Technology (AT), such as screen
    /// readers, when exporting to PDF, but not for sighted readers of your file.
    ///
    /// This field is intended for instructions that help the user navigate the
    /// table using AT. It is not an alternative description, so do not duplicate
    /// the contents of the table within. Likewise, do not use this for the core
    /// takeaway of the table. Instead, include that in the text around the table
    /// or, even better, in a @figure.caption[figure caption].
    ///
    /// If in doubt whether your table is complex enough to warrant a summary, err
    /// on the side of not including one. If you are certain that your table is
    /// complex enough, consider whether a sighted user might find it challenging.
    /// They might benefit from the instructions you put here, so consider printing
    /// them visibly in the document instead.
    ///
    /// The API of this feature is temporary. Hence, calling this function requires
    /// enabling the `a11y-extras` feature flag at the moment. Even if this
    /// functionality should be available without a feature flag in the future, the
    /// summary will remain exclusive to PDF export.
    ///
    /// ```example
    /// #figure(
    ///   pdf.table-summary(
    ///     // The summary just provides orientation and structural
    ///     // information for AT users.
    ///     summary: "The first two columns list the names of each participant. The last column contains cells spanning multiple rows for their assigned group.",
    ///     table(
    ///       columns: 3,
    ///       table.header[First Name][Given Name][Group],
    ///       [Mike], [Davis], table.cell(rowspan: 3)[Sales],
    ///       [Anna], [Smith],
    ///       [John], [Johnson],
    ///       [Sara], [Wilkins], table.cell(rowspan: 2)[Operations],
    ///       [Tom], [Brown],
    ///     ),
    ///   ),
    ///   // This is the key takeaway of the table, so we put it in the caption.
    ///   caption: [The Sales org now has a new member],
    /// )
    /// ```
    #[func(since = "0.14.0")]
    #[feature = A11yExtras]
    pub fn table_summary(
        #[named] summary: Option<EcoString>,
        /// The table.
        table: TableElem,
    ) -> Content {
        table.with_summary(summary).pack()
    }

    /// Explicitly defines a cell as a header cell.
    ///
    /// Header cells help users of Assistive Technology (AT) understand and navigate
    /// complex tables. When your table is correctly marked up with header cells, AT
    /// can announce the relevant header information on-demand when entering a cell.
    ///
    /// By default, Typst will automatically mark all cells within @table.header as
    /// header cells. They will apply to the columns below them. You can use that
    /// function's @table.header.level[`level`] parameter to make header cells
    /// labelled by other header cells.
    ///
    /// The `pdf.header-cell` function allows you to indicate that a cell is a
    /// header cell in the following additional situations:
    ///
    /// - You have a *header column* in which each cell applies to its row. In that
    ///   case, you pass `{"row"}` as an argument to the
    ///   @pdf.header-cell.scope[`scope` parameter] to indicate that the header cell
    ///   applies to the row.
    /// - You have a cell in @table.header, for example at the very start, that
    ///   labels both its row and column. In that case, you pass `{"both"}` as an
    ///   argument to the @pdf.header-cell.scope[`scope`] parameter.
    /// - You have a header cell in a row not containing other header cells. In that
    ///   case, you can use this function to mark it as a header cell.
    ///
    /// The API of this feature is temporary. Hence, calling this function requires
    /// enabling the `a11y-extras` feature flag at the moment. In a future Typst
    /// release, this functionality may move out of the `pdf` module so that tables
    /// in other export targets can contain the same information.
    ///
    /// ```example
    /// >>> #set text(font: "IBM Plex Sans")
    /// #show table.cell.where(x: 0): set text(weight: "medium")
    /// #show table.cell.where(y: 0): set text(weight: "bold")
    ///
    /// #table(
    ///   columns: 3,
    ///   align: (start, end, end),
    ///
    ///   table.header(
    ///     // Top-left cell: Labels both the nutrient rows
    ///     // and the serving size columns.
    ///     pdf.header-cell(scope: "both")[Nutrient],
    ///     [Per 100g],
    ///     [Per Serving],
    ///   ),
    ///
    ///   // First column cells are row headers
    ///   pdf.header-cell(scope: "row")[Calories],
    ///   [250 kcal], [375 kcal],
    ///   pdf.header-cell(scope: "row")[Protein],
    ///   [8g], [12g],
    ///   pdf.header-cell(scope: "row")[Fat],
    ///   [12g], [18g],
    ///   pdf.header-cell(scope: "row")[Carbs],
    ///   [30g], [45g],
    /// )
    /// ```
    #[func(since = "0.14.0")]
    #[feature = A11yExtras]
    pub fn header_cell(
        /// The nesting level of this header cell.
        #[named]
        #[default(NonZeroU32::ONE)]
        level: NonZeroU32,
        /// What track of the table this header cell applies to.
        #[named]
        #[default]
        scope: TableHeaderScope,
        /// The table cell.
        ///
        /// This can be content or a call to @table.cell.
        cell: TableCell,
    ) -> Content {
        cell.with_kind(Smart::Custom(TableCellKind::Header(level, scope)))
            .pack()
    }

    /// Explicitly defines this cell as a data cell.
    ///
    /// Each cell in a table is either a header cell or a data cell. By default, all
    /// cells in @table.header are header cells, and all other cells data cells.
    ///
    /// If your header contains a cell that is not a header cell, you can use this
    /// function to mark it as a data cell.
    ///
    /// The API of this feature is temporary. Hence, calling this function requires
    /// enabling the `a11y-extras` feature flag at the moment. In a future Typst
    /// release, this functionality may move out of the `pdf` module so that tables
    /// in other export targets can contain the same information.
    ///
    /// ```example
    /// #show table.cell.where(x: 0): set text(weight: "bold")
    /// #show table.cell.where(x: 1): set text(style: "italic")
    /// #show table.cell.where(x: 1, y: 0): set text(style: "normal")
    ///
    /// #table(
    ///   columns: 3,
    ///   align: (left, left, center),
    ///
    ///   table.header[Objective][Key Result][Status],
    ///
    ///   table.header(
    ///     level: 2,
    ///     table.cell(colspan: 2)[Improve Customer Satisfaction],
    ///     // Status is data for this objective, not a header
    ///     pdf.data-cell[✓ On Track],
    ///   ),
    ///   [], [Increase NPS to 50+], [45],
    ///   [], [Reduce churn to \<5%], [4.2%],
    ///
    ///   table.header(
    ///     level: 2,
    ///     table.cell(colspan: 2)[Grow Revenue],
    ///     pdf.data-cell[⚠ At Risk],
    ///   ),
    ///   [], [Achieve \$2M ARR], [\$1.8M],
    ///   [], [Close 50 enterprise deals], [38],
    /// )
    /// ```
    #[func(since = "0.14.0")]
    #[feature = A11yExtras]
    pub fn data_cell(
        /// The table cell.
        ///
        /// This can be content or a call to @table.cell.
        cell: TableCell,
    ) -> Content {
        cell.with_kind(Smart::Custom(TableCellKind::Data)).pack()
    }
}

/// Encapsulates a list of compatible PDF standards.
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PdfStandards {
    pub(crate) config: krilla::configure::Configuration,
}

cast! {
    PdfStandards,
    self => {
        Value::Array(Array::from_iter(self.standards().map(IntoValue::into_value)))
    },
    standard: PdfStandard => PdfStandards::new([standard])?,
    values: Array => {
        let list = values.into_iter().map(Value::cast).collect::<HintedStrResult<Vec<PdfStandard>>>()?;
        Self::new(list)?
    }
}

impl PdfStandards {
    /// Validates a list of PDF standards for compatibility and returns their
    /// encapsulated representation.
    pub fn new<T: Into<PdfStandard>>(
        list: impl IntoIterator<Item = T>,
    ) -> HintedStrResult<Self> {
        use krilla::configure::{
            Accessibility, Archival, ConfigurationBuilder, ConfigurationError, PdfVersion,
        };

        use crate::util::ValidatorsExt;

        let mut version: Option<PdfVersion> = None;
        let mut set_version = |v: PdfVersion| -> StrResult<()> {
            if let Some(prev) = version {
                bail!(
                    "PDF cannot conform to {} and {} at the same time",
                    prev.as_str(),
                    v.as_str(),
                );
            }
            version = Some(v);
            Ok(())
        };

        let mut archival_validator = None;
        let mut set_archival_validator = |a: Archival| -> StrResult<()> {
            if archival_validator.is_some() {
                bail!("choose at most one PDF/A standard");
            }
            archival_validator = Some(a);
            Ok(())
        };

        let mut accessibility_validator = None;
        let mut set_accessibility_validator = |ua: Accessibility| -> StrResult<()> {
            if accessibility_validator.is_some() {
                bail!("choose at most one PDF/UA standard");
            }
            accessibility_validator = Some(ua);
            Ok(())
        };

        for standard in list {
            match standard.into() {
                PdfStandard::V_1_4 => set_version(PdfVersion::Pdf14)?,
                PdfStandard::V_1_5 => set_version(PdfVersion::Pdf15)?,
                PdfStandard::V_1_6 => set_version(PdfVersion::Pdf16)?,
                PdfStandard::V_1_7 => set_version(PdfVersion::Pdf17)?,
                PdfStandard::V_2_0 => set_version(PdfVersion::Pdf20)?,
                PdfStandard::A_1b => set_archival_validator(Archival::A1_B)?,
                PdfStandard::A_1a => set_archival_validator(Archival::A1_A)?,
                PdfStandard::A_2b => set_archival_validator(Archival::A2_B)?,
                PdfStandard::A_2u => set_archival_validator(Archival::A2_U)?,
                PdfStandard::A_2a => set_archival_validator(Archival::A2_A)?,
                PdfStandard::A_3b => set_archival_validator(Archival::A3_B)?,
                PdfStandard::A_3u => set_archival_validator(Archival::A3_U)?,
                PdfStandard::A_3a => set_archival_validator(Archival::A3_A)?,
                PdfStandard::A_4 => set_archival_validator(Archival::A4)?,
                PdfStandard::A_4f => set_archival_validator(Archival::A4F)?,
                PdfStandard::A_4e => set_archival_validator(Archival::A4E)?,
                PdfStandard::UA_1 => set_accessibility_validator(Accessibility::UA1)?,
            }
        }

        let mut builder = ConfigurationBuilder::new();

        if let Some(version) = version {
            builder = builder.with_version(version);
        }

        if let Some(archival_validator) = archival_validator {
            builder = builder.with_archival_validator(archival_validator);
        }

        if let Some(accessibility_validator) = accessibility_validator {
            builder = builder.with_accessibility_validator(accessibility_validator);
        }

        let config = builder.finish().map_err(|e| {
            let (message, validators) = match e {
                ConfigurationError::NoOverlappingValidatorsRange(validators) => {
                    let list = validators.to_and_list();
                    let message = eco_format!(
                        "{list} are mutually incompatible because \
                         they do not have any overlapping PDF versions"
                    );
                    (message, validators)
                }
                ConfigurationError::VersionDoesNotMatchValidatorsRange(
                    version,
                    validators,
                ) => {
                    let list = validators.to_and_list();
                    let message =
                        eco_format!("{} is not compatible with {list}", version.as_str());
                    (message, validators)
                }
            };
            HintedString::new(message)
                .with_hints(validators.into_iter().map(version_hint))
        })?;

        Ok(Self { config })
    }

    /// Returns an iterator over PDF standards
    pub fn standards(&self) -> impl Iterator<Item = PdfStandard> {
        std::iter::once(self.config.version().into())
            .chain(self.config.validators().into_iter().map(Into::into))
    }
}

/// A hint specifying which PDF version a validator is compatible with.
fn version_hint(validator: krilla::configure::Validator) -> EcoString {
    let min = validator.min();
    let max = validator.max();
    if let Some(min) = min {
        if min == max {
            eco_format!("{} requires version {}", validator.as_str(), min.as_str())
        } else {
            eco_format!(
                "{} requires a version between {} and {}",
                validator.as_str(),
                min.as_str(),
                max.as_str()
            )
        }
    } else {
        eco_format!("{} requires at least {}", validator.as_str(), max.as_str())
    }
}

impl Debug for PdfStandards {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.pad("PdfStandards(..)")
    }
}

impl Default for PdfStandards {
    fn default() -> Self {
        use krilla::configure::{ConfigurationBuilder, PdfVersion};
        Self {
            config: ConfigurationBuilder::new()
                .with_version(PdfVersion::Pdf17)
                .finish()
                .unwrap(),
        }
    }
}

/// A PDF standard that Typst can enforce conformance with.
///
/// Support for more standards is planned.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast, Serialize, Deserialize)]
#[allow(non_camel_case_types)]
#[non_exhaustive]
pub enum PdfStandard {
    /// PDF 1.4.
    #[string("1.4")]
    #[serde(rename = "1.4")]
    V_1_4,
    /// PDF 1.5.
    #[string("1.5")]
    #[serde(rename = "1.5")]
    V_1_5,
    /// PDF 1.6.
    #[string("1.6")]
    #[serde(rename = "1.6")]
    V_1_6,
    /// PDF 1.7.
    #[string("1.7")]
    #[serde(rename = "1.7")]
    V_1_7,
    /// PDF 2.0.
    #[string("2.0")]
    #[serde(rename = "2.0")]
    V_2_0,
    /// PDF/A-1b.
    #[string("a-1b")]
    #[serde(rename = "a-1b")]
    A_1b,
    /// PDF/A-1a.
    #[string("a-1a")]
    #[serde(rename = "a-1a")]
    A_1a,
    /// PDF/A-2b.
    #[string("a-2b")]
    #[serde(rename = "a-2b")]
    A_2b,
    /// PDF/A-2u.
    #[string("a-2u")]
    #[serde(rename = "a-2u")]
    A_2u,
    /// PDF/A-2a.
    #[string("a-2a")]
    #[serde(rename = "a-2a")]
    A_2a,
    /// PDF/A-3b.
    #[string("a-3b")]
    #[serde(rename = "a-3b")]
    A_3b,
    /// PDF/A-3u.
    #[string("a-3u")]
    #[serde(rename = "a-3u")]
    A_3u,
    /// PDF/A-3a.
    #[string("a-3a")]
    #[serde(rename = "a-3a")]
    A_3a,
    /// PDF/A-4.
    #[string("a-4")]
    #[serde(rename = "a-4")]
    A_4,
    /// PDF/A-4f.
    #[string("a-4f")]
    #[serde(rename = "a-4f")]
    A_4f,
    /// PDF/A-4e.
    #[string("a-4e")]
    #[serde(rename = "a-4e")]
    A_4e,
    /// PDF/UA-1.
    #[string("ua-1")]
    #[serde(rename = "ua-1")]
    UA_1,
}

impl From<PdfVersion> for PdfStandard {
    fn from(value: PdfVersion) -> Self {
        match value {
            PdfVersion::Pdf14 => PdfStandard::V_1_4,
            PdfVersion::Pdf15 => PdfStandard::V_1_5,
            PdfVersion::Pdf16 => PdfStandard::V_1_6,
            PdfVersion::Pdf17 => PdfStandard::V_1_7,
            PdfVersion::Pdf20 => PdfStandard::V_2_0,
        }
    }
}

impl From<Validator> for PdfStandard {
    fn from(value: Validator) -> Self {
        match value {
            Validator::A(archival) => match archival {
                Archival::A1_A => PdfStandard::A_1a,
                Archival::A1_B => PdfStandard::A_1b,
                Archival::A2_A => PdfStandard::A_2a,
                Archival::A2_B => PdfStandard::A_2b,
                Archival::A2_U => PdfStandard::A_2u,
                Archival::A3_A => PdfStandard::A_3a,
                Archival::A3_B => PdfStandard::A_3b,
                Archival::A3_U => PdfStandard::A_3u,
                Archival::A4 => PdfStandard::A_4,
                Archival::A4F => PdfStandard::A_4f,
                Archival::A4E => PdfStandard::A_4e,
            },
            Validator::Ua(accessibility) => match accessibility {
                Accessibility::UA1 => PdfStandard::UA_1,
            },
        }
    }
}

/// Document settings for PDF export.
#[derive(Debug, Default, Clone, Eq, PartialEq, Hash)]
pub struct PdfFormatOptions<F: Fields = Complete> {
    /// Specifies which ranges of pages should be included in the PDF. When
    /// `None`, all pages should be exported.
    pub pages: F::Value<PdfFormat, { PdfFormat::pages.index() }>,
    /// A list of PDF standards that Typst will enforce conformance with.
    pub standard: F::Value<PdfFormat, { PdfFormat::standard.index() }>,
    /// By default, even when not producing a `PDF/UA-1` document, a tagged PDF
    /// document is written to provide a baseline of accessibility. In some
    /// circumstances, for example when trying to reduce the size of a document,
    /// it can be desirable to disable tagged PDF.
    pub tagged: F::Value<PdfFormat, { PdfFormat::tagged.index() }>,
    /// Wether to format the PDF in a human readable way.
    pub pretty: F::Value<PdfFormat, { PdfFormat::pretty.index() }>,
}

impl Populate for PdfFormatOptions {
    fn populate(&mut self, styles: Spanned<StyleChain>) {
        // VOLATILE: This must be updated when adding more fields.
        self.pages.populate(styles);
        self.standard.populate(styles);
        self.tagged.populate(styles);
        self.pretty.populate(styles);
    }
}

impl PdfFormatOptions<Partial> {
    /// Resolves the [`Partial`] options to [`Complete`] ones, given defaults.
    pub fn resolve(&self, default: &PdfFormatOptions) -> PdfFormatOptions {
        PdfFormatOptions {
            pages: Partial::resolve_cloned(&self.pages, &default.pages),
            standard: Partial::resolve_cloned(&self.standard, &default.standard),
            tagged: Partial::resolve(self.tagged, default.tagged),
            pretty: Partial::resolve(self.pretty, default.pretty),
        }
    }
}

/// A file that will be attached to the output PDF.
///
/// This can be used to distribute additional files associated with the PDF
/// within it. PDF readers will display the files in a file listing.
///
/// Some international standards use this mechanism to attach machine-readable
/// data (e.g., ZUGFeRD/Factur-X for invoices) that mirrors the visual content
/// of the PDF.
///
/// = Example <example>
/// ```typ
/// #pdf.attach(
///   "experiment.csv",
///   relationship: "supplement",
///   mime-type: "text/csv",
///   description: "Raw Oxygen readings from the Arctic experiment",
/// )
/// ```
///
/// = Notes <notes>
/// - This element is ignored if exporting to a format other than PDF.
/// - File attachments are not currently supported for PDF/A-2, even if the
///   attached file conforms to PDF/A-1 or PDF/A-2.
#[elem(since = "0.14.0", keywords = ["embed"], Locatable)]
pub struct AttachElem {
    /// The path of the file to be attached.
    ///
    /// Must always be specified, but is only read from if no data is provided
    /// in the following argument.
    #[required]
    #[parse(
        let Spanned { v: path, span } =
            args.expect::<Spanned<PathOrStr>>("path")?;
        let resolved = path.resolve_if_some(span.id()).at(span)?;
        // The derived part is the virtual-root-relative resolved path.
        let derived = resolved.vpath().get_without_slash().into();
        Derived::new(path, derived)
    )]
    pub path: Derived<PathOrStr, EcoString>,

    /// Raw file data, optionally.
    ///
    /// If omitted, the data is read from the specified path.
    #[positional]
    // Not actually required as an argument, but always present as a field.
    // We can't distinguish between the two at the moment.
    #[required]
    #[parse(
        match args.eat::<Bytes>()? {
            Some(data) => data,
            None => engine.world.file(resolved.intern()).at(span)?,
        }
    )]
    pub data: Bytes,

    /// The relationship of the attached file to the document.
    ///
    /// Ignored if export doesn't target PDF/A-3.
    pub relationship: Option<AttachedFileRelationship>,

    /// The MIME type of the attached file.
    // TODO: Use `krilla::embed::MimeType`, or a wrapper around it here.
    // The problem is that it's currently completely opaque and doesn't allow
    // retrieving the inner string after construction.
    pub mime_type: Option<EcoString>,

    /// A description for the attached file.
    pub description: Option<EcoString>,
}

/// The relationship of an attached file with the document.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Cast)]
pub enum AttachedFileRelationship {
    /// The PDF document was created from the source file.
    Source,
    /// The file was used to derive a visual presentation in the PDF.
    Data,
    /// An alternative representation of the document.
    Alternative,
    /// Additional resources for the document.
    Supplement,
}
