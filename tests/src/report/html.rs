use std::collections::hash_map::Entry;
use std::fmt::Display;
use std::fmt::Write as _;

use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;

use crate::collect::{FileSize, TestOutput};
use crate::report::diff::{FileDiff, Image, Line, LineKind, Lines, TextSpan};
use crate::report::html::icons::SvgIcon;
use crate::report::{DiffKind, ReportFile, TestReport};

static REPORT_STYLE: &str = include_str!("report.css");
static REPORT_SCRIPT: &str = include_str!("report.js");

macro_rules! display {
    ($($arg:tt)*) => {
        ::typst_utils::display(move |f| write!(f, $($arg)*))
    }
}

/// A HTML writer.
struct Html {
    /// A map from the svg icon path to a cached id.
    svg_icon_cache: FxHashMap<SvgIcon, EcoString>,
    in_attribute_list: bool,
    buf: String,
}

impl Html {
    fn new() -> Self {
        Self {
            svg_icon_cache: FxHashMap::default(),
            in_attribute_list: false,
            buf: String::from("<!DOCTYPE html>\n"),
        }
    }

    fn finish(self) -> String {
        self.buf
    }

    fn elem(&mut self, name: &'static str) -> HtmlElem<'_> {
        HtmlElem::new(self, name)
    }
}

/// This is the low-level API for manipulating the text buffer.
impl Html {
    fn start_element(&mut self, name: &str) {
        if self.in_attribute_list {
            self.buf.push('>');
        }

        write!(self.buf, "<{name}").ok();

        self.in_attribute_list = true;
    }

    fn write_attr(&mut self, name: &str, val: impl Display) {
        assert!(self.in_attribute_list);

        let val = EscFmt::new(val, escape_attr);
        write!(self.buf, r#" {name}="{val}""#).ok();
    }

    fn write_text(&mut self, text: impl Display) {
        if self.in_attribute_list {
            self.buf.push('>');
            self.in_attribute_list = false;
        }

        let text = EscFmt::new(text, escape_text);
        write!(self.buf, "{text}").ok();
    }

    fn write_raw_text(&mut self, text: impl Display) {
        if self.in_attribute_list {
            self.buf.push('>');
            self.in_attribute_list = false;
        }

        write!(self.buf, "{text}").ok();
    }

    fn end_element(&mut self, name: &str, void: bool) {
        if self.in_attribute_list {
            self.buf.push('>');
            self.in_attribute_list = false;
        }
        if !void {
            write!(self.buf, "</{name}>").ok();
        }
    }
}

/// A writer that allows escaping ASCII characters.
struct EscWriter<W, F> {
    writer: W,
    escape: F,
}

impl<W, F> EscWriter<W, F> {
    fn new(writer: W, escape: F) -> Self {
        Self { writer, escape }
    }
}

impl<W, F> std::fmt::Write for EscWriter<W, F>
where
    W: std::fmt::Write,
    F: Fn(u8) -> Option<&'static str>,
{
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let mut remainder = s;
        while let Some((idx, esc)) = remainder
            .bytes()
            .enumerate()
            .find_map(|(i, b)| (self.escape)(b).map(|esc| (i, esc)))
        {
            self.writer.write_str(&remainder[..idx])?;
            self.writer.write_str(esc)?;

            remainder = &remainder[idx + 1..];
        }
        self.writer.write_str(remainder)?;
        Ok(())
    }
}

/// A wrapper around a value that makes use of [`EscWriter`] to escape ASCII
/// characters.
struct EscFmt<T, F> {
    val: T,
    escape: F,
}

impl<T, F> EscFmt<T, F> {
    fn new(val: T, escape: F) -> Self {
        Self { val, escape }
    }
}

impl<T, F> std::fmt::Display for EscFmt<T, F>
where
    T: Display,
    F: Fn(u8) -> Option<&'static str> + Copy,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(EscWriter::new(f, self.escape), "{}", self.val)
    }
}

/// Escape text inside a HTML element.
fn escape_text(b: u8) -> Option<&'static str> {
    match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        _ => None,
    }
}

/// Escape text inside a HTML attribute.
fn escape_attr(b: u8) -> Option<&'static str> {
    match b {
        b'&' => Some("&amp;"),
        b'"' => Some("&quot;"),
        _ => None,
    }
}

/// A HTML element, that automatically calls [`Html::start_element`] when
/// created and [`Html::end_element`] when dropped.
struct HtmlElem<'a> {
    html: &'a mut Html,
    name: &'static str,
    /// Void elements must not have closing tags.
    void: bool,
}

impl<'a> HtmlElem<'a> {
    fn new(html: &'a mut Html, name: &'static str) -> Self {
        html.start_element(name);
        Self { html, name, void: false }
    }

    fn new_void(html: &'a mut Html, name: &'static str) -> Self {
        html.start_element(name);
        Self { html, name, void: true }
    }

    fn elem(&mut self, name: &'static str) -> HtmlElem<'_> {
        HtmlElem::new(self.html, name)
    }

    fn void_elem(&mut self, name: &'static str) -> HtmlElem<'_> {
        HtmlElem::new_void(self.html, name)
    }

    fn attr(&mut self, name: &str, val: impl Display) -> &mut Self {
        self.html.write_attr(name, val);
        self
    }

    fn opt_attr(&mut self, name: &str, val: Option<impl Display>) -> &mut Self {
        if let Some(val) = val {
            self.html.write_attr(name, val);
        }
        self
    }

    fn text(&mut self, text: impl Display) -> &mut Self {
        self.html.write_text(text);
        self
    }

    fn raw_text(&mut self, text: impl Display) -> &mut Self {
        self.html.write_raw_text(text);
        self
    }

    fn text_opt(&mut self, text: Option<impl Display>) -> &mut Self {
        if let Some(text) = text {
            self.html.write_text(text);
        }
        self
    }

    fn with<'e, T>(&'e mut self, f: impl FnOnce(&'e mut Self) -> T) -> T {
        f(self)
    }
}

impl Drop for HtmlElem<'_> {
    fn drop(&mut self) {
        self.html.end_element(self.name, self.void);
    }
}

macro_rules! elem_methods {
    ($(fn $name:ident();)+) => {
        $(
            fn $name(&mut self) -> HtmlElem<'_> {
                self.elem(stringify!($name))
            }
        )+
    };
}

macro_rules! void_elem_methods {
    ($(fn $name:ident();)+) => {
        $(
            fn $name(&mut self) -> HtmlElem<'_> {
                self.void_elem(stringify!($name))
            }
        )+
    };
}

macro_rules! attr_methods {
    ($(fn $name:ident($val_ty:ty);)+) => {
        $(
            fn $name(&mut self, $name: $val_ty) -> &mut Self {
                self.attr(stringify!($name), $name)
            }
        )+
    };
}

macro_rules! flag_attr_methods {
    ($(fn $name:ident();)+) => {
        $(
            fn $name(&mut self, $name: bool) -> &mut Self {
                self.opt_attr(stringify!($name), $name.then_some(stringify!($name)))
            }
        )+
    };
}

/// Convenience methods.
impl HtmlElem<'_> {
    elem_methods! {
        fn h2();
        fn h3();
        fn div();
        fn fieldset();
        fn label();
        fn button();
        fn a();

        fn canvas();

        fn ul();
        fn li();

        fn table();
        fn colgroup();
        fn col();
        fn tr();
        fn td();

        fn pre();
        fn span();
        fn del();
        fn ins();
    }

    void_elem_methods! {
        fn meta();
        fn input();
        fn img();
    }

    attr_methods! {
        fn id(impl Display);
        fn name(impl Display);
        fn class(impl Display);
        fn title(&str);
        fn placeholder(&str);

        fn href(impl Display);
        fn target(impl Display);

        fn src(impl Display);

        fn value(impl Display);
        fn min(f32);
        fn max(f32);
        fn step(f32);

        fn colspan(u32);

        fn tabindex(i32);
    }

    flag_attr_methods! {
        fn disabled();
        fn checked();
        fn hidden();
    }

    fn type_(&mut self, ty: &str) -> &mut Self {
        self.attr("type", ty)
    }

    fn data_attr(&mut self, name: &str, value: impl Display) -> &mut Self {
        let name = format!("data-{name}");
        self.attr(&name, value)
    }

    fn aria_role(&mut self, role: &str) -> &mut Self {
        self.attr("aria-role", role)
    }

    /// The corresponding tab of a `aria-role="tabpanel"`.
    fn aria_labelledby(&mut self, tab_id: impl Display) -> &mut Self {
        self.attr("aria-labelledby", tab_id)
    }

    /// Set for `aria-role="tab"` when the tab is selected.
    fn aria_selected(&mut self, selected: bool) -> &mut Self {
        self.attr("aria-selected", selected)
    }

    fn aria_expanded(&mut self, expanded: bool) -> &mut Self {
        self.attr("aria-expanded", expanded)
    }

    fn aria_controls(&mut self, id: impl Display) -> &mut Self {
        self.attr("aria-controls", id)
    }
}

/// Generate a HTML test report.
pub fn generate(reports: &[TestReport]) -> String {
    let mut html = Html::new();

    html.elem("html").attr("lang", "en").with(|root| {
        root.elem("head").with(|head| {
            head.meta().attr("charset", "utf-8");
            head.elem("title").text("Typst test report");
            head.elem("style").text(REPORT_STYLE);
        });

        root.elem("body").with(|body| {
            test_reports(body, reports);

            //
            body.elem("script").type_("text/javascript").raw_text(REPORT_SCRIPT);
        });
    });

    html.finish()
}

/// Deduplicates SVG icons by means of the SVG `use` element and the fact that
/// IDs, even inside of SVGs are global inside a HTML document.
fn svg_icon(parent: &mut HtmlElem, icon: SvgIcon) {
    let n = parent.html.svg_icon_cache.len();
    parent
        .elem("svg")
        .attr("xmlns", "http://www.w3.org/2000/svg")
        .attr("width", "16")
        .attr("height", "16")
        .attr("fill", "currentColor")
        .with(|svg| match svg.html.svg_icon_cache.entry(icon) {
            Entry::Occupied(occupied) => {
                let id = occupied.get().clone();
                svg.elem("use").href(display!("#{id}"));
            }
            Entry::Vacant(vacant) => {
                let id = eco_format!("svg-icon-{n}");
                vacant.insert(id.clone());
                svg.elem("path").id(&id).attr("d", icon.as_str());
            }
        });
}

fn icon_button(
    parent: &mut HtmlElem,
    id: impl Display,
    title: &str,
    icon: SvgIcon,
    disabled: bool,
) {
    parent
        .button()
        .class("icon-button")
        .id(id)
        .title(title)
        .disabled(disabled)
        .with(|button| svg_icon(button, icon));
}

fn tab_icon_button(
    parent: &mut HtmlElem,
    class: &str,
    disambiguator: usize,
    value: impl Display + Copy,
    title: &str,
    icon: SvgIcon,
    checked: bool,
) {
    parent.label().class("icon-toggle-button").with(|label| {
        label
            .input()
            .id(display!("{class}-{disambiguator}-{value}"))
            .type_("radio")
            .aria_role("tab")
            .class(class)
            .name(display!("{class}-{disambiguator}"))
            .title(title)
            .value(value)
            .checked(checked)
            .aria_selected(checked);
        svg_icon(label, icon);
    });
}

fn test_reports(body: &mut HtmlElem, reports: &[TestReport]) {
    body.div().class("container").with(|div| {
        div.div().class("sidebar").with(|div| {
            sidebar(div, reports);
        });

        div.div().class("diff-container").tabindex(-1).with(|div| {
            div.h2().class("diff-container-header").text("Changes");

            for (test_idx, test_report) in reports.iter().enumerate() {
                let close = (test_report.files.first())
                    .and_then(|report_file| report_file.diffs.first())
                    .is_some_and(|diff| match diff {
                        DiffKind::Text(diff) => is_large_text_diff(diff),
                        DiffKind::Image(_) => false,
                    });

                div.div()
                    .id(display!("r-{}", test_report.name))
                    .class("test-report")
                    .with(|div| {
                        div.div().class("test-report-header").with(|div| {
                            test_report_header(div, test_report, test_idx, close);
                        });

                        div.div()
                            .id(display!("test-report-body-{test_idx}"))
                            .class("test-report-body")
                            .hidden(close)
                            .with(|div| {
                                for (file_idx, report_file) in
                                    test_report.files.iter().enumerate()
                                {
                                    report_file_tab_panel(
                                        div,
                                        report_file,
                                        test_idx,
                                        file_idx,
                                    );
                                }
                            });
                    });
            }

            if reports.is_empty() {
                div.div().class("diff-container-empty").text("NONE");
            }

            div.div().class("diff-scroll-padding");
        });
    })
}

fn sidebar(parent: &mut HtmlElem, reports: &[TestReport]) {
    parent.h2().text("Filter");

    parent.div().class("sidebar-setting").with(|div| {
        div.fieldset().class("control-group flex-grow").with(|fieldset| {
            fieldset
                .input()
                .type_("search")
                .id("filter-search")
                .class("search-field")
                .placeholder("Search…");
            // This button doesn't do anything, but clicking it will
            // unfocus the search field and trigger the change event :)
            icon_button(fieldset, "search-button", "Search", icons::SEARCH, false);
        });
    });

    parent.div().class("sidebar-setting").with(|div| {
        let checkbox_icon_button = |parent: &mut HtmlElem, output, disabled| {
            let (title, icon) = match output {
                TestOutput::Render => ("Filter PNG", icons::RENDER),
                TestOutput::Pdf => ("Filter PDF", icons::PDF),
                TestOutput::Pdftags => ("Filter PDF tags", icons::PDFTAGS),
                TestOutput::Svg => ("Filter SVG", icons::SVG),
                TestOutput::Html => ("Filter HTML", icons::HTML),
            };
            parent.label().class("icon-toggle-button").with(|label| {
                label
                    .input()
                    .type_("checkbox")
                    .id(display!("filter-diff-format-{output}"))
                    .title(title)
                    .value(output)
                    .disabled(disabled);
                svg_icon(label, icon);
            });
        };

        div.text("Diff Format");
        div.fieldset().class("control-group").with(|fieldset| {
            for &output in TestOutput::ALL.iter() {
                let enabled = (reports.iter())
                    .flat_map(|report| report.files.iter())
                    .any(|file| file.output == output);

                checkbox_icon_button(fieldset, output, !enabled);
            }
        });
    });

    parent.h2().text("Settings");

    parent.div().class("sidebar-setting").with(|div| {
        div.text("Diff Format");
        div.fieldset().class("control-group").with(|fieldset| {
            for &output in TestOutput::ALL.iter() {
                let id = display!("global-diff-format-{output}");
                let (title, icon) = match output {
                    TestOutput::Render => ("Show PNG diffs", icons::RENDER),
                    TestOutput::Pdf => ("Show PDF diffs", icons::PDF),
                    TestOutput::Pdftags => ("Show PDF tags diffs", icons::PDFTAGS),
                    TestOutput::Svg => ("Show SVG diffs", icons::SVG),
                    TestOutput::Html => ("Show HTML diffs", icons::HTML),
                };

                let enabled = (reports.iter())
                    .flat_map(|report| report.files.iter())
                    .any(|file| file.output == output);

                icon_button(fieldset, id, title, icon, !enabled);
            }
        });
    });

    parent.div().class("sidebar-setting").with(|div| {
        div.text("Diff Mode");
        div.fieldset().class("control-group").with(|fieldset| {
            icon_button(
                fieldset,
                "global-diff-mode-image",
                "Show image diffs",
                icons::IMAGE,
                false,
            );
            icon_button(
                fieldset,
                "global-diff-mode-text",
                "Show text diffs",
                icons::TEXT,
                false,
            );
        });
    });

    parent.div().class("sidebar-setting").with(|div| {
        div.text("Image View Mode");
        div.fieldset().class("control-group").with(|fieldset| {
            icon_button(
                fieldset,
                "global-image-view-mode-side-by-side",
                "Show Image View Mode side by side",
                icons::VIEW_SIDE_BY_SIDE,
                false,
            );
            icon_button(
                fieldset,
                "global-image-view-mode-blend",
                "Show Image View Mode blend",
                icons::VIEW_BLEND,
                false,
            );
            icon_button(
                fieldset,
                "global-image-view-mode-difference",
                "Show Image View Mode difference",
                icons::VIEW_DIFFERENCE,
                false,
            );
        });
    });

    parent.h2().text("Tests");

    parent.ul().class("sidebar-list").tabindex(-1).with(|ul| {
        for report in reports.iter() {
            ul.li().with(|li| {
                li.a()
                    .href(display!("#r-{}", report.name))
                    .title(&report.name)
                    .text(&report.name);
            });
        }
        if reports.is_empty() {
            ul.div().class("sidebar-list-empty").text("NONE");
        }
    });
}

fn test_report_header(
    parent: &mut HtmlElem,
    test_report: &TestReport,
    test_idx: usize,
    close: bool,
) {
    parent
        .button()
        .class("test-report-toggle icon-button")
        .aria_expanded(!close)
        .aria_controls(display!("test-report-body-{test_idx}"))
        .title("Toggle report")
        .with(|button| {
            svg_icon(button, icons::CHEVRON_DOWN);
        });

    parent.h3().with(|h3| {
        // Allow pressing the name of the test to scroll to the top of the diff.
        h3.a()
            .href(display!("#r-{}", test_report.name))
            .text(&test_report.name);
    });

    parent.div().class("flex-grow");

    for (file_idx, report_file) in test_report.files.iter().enumerate() {
        parent
            .div()
            .class("report-file-header")
            .hidden(file_idx != 0)
            .with(|div| {
                report_file_header(div, report_file);
            });
    }

    parent
        .fieldset()
        .aria_role("tablist")
        .class("report-file-tab-group control-group")
        .with(|fieldset| {
            for (file_idx, report_file) in test_report.files.iter().enumerate() {
                let (title, icon) = match report_file.output {
                    TestOutput::Render => ("View PNG", icons::RENDER),
                    TestOutput::Pdf => ("View PDF", icons::PDF),
                    TestOutput::Pdftags => ("View PDF tags", icons::PDFTAGS),
                    TestOutput::Svg => ("View SVG", icons::SVG),
                    TestOutput::Html => ("View HTML", icons::HTML),
                };
                let report_file_tab = |parent: &mut HtmlElem| {
                    tab_icon_button(
                        parent,
                        "report-file-tab",
                        test_idx,
                        report_file.output,
                        title,
                        icon,
                        file_idx == 0,
                    )
                };

                if report_file.diffs.len() == 1 {
                    report_file_tab(fieldset);
                } else {
                    fieldset.div().class("report-file-tab-wrapper").with(|div| {
                        report_file_tab(div);

                        div.div().class("file-diff-tab-group-wrapper").with(|div| {
                            file_diff_tabs(div, report_file, test_idx, file_idx);
                        });
                    });
                }
            }
        });
}

fn file_diff_tabs(
    parent: &mut HtmlElem,
    report_file: &ReportFile,
    test_idx: usize,
    file_idx: usize,
) {
    let n = test_idx * TestOutput::ALL.len() + file_idx;
    parent
        .fieldset()
        .aria_role("tablist")
        .class("file-diff-tab-group control-group")
        .with(|fieldset| {
            for (diff_idx, diff) in report_file.diffs.iter().enumerate() {
                let (title, icon) = match diff {
                    DiffKind::Image(_) => ("View image diff", icons::IMAGE),
                    DiffKind::Text(_) => ("View text diff", icons::TEXT),
                };
                tab_icon_button(
                    fieldset,
                    "file-diff-tab",
                    n,
                    diff.kind_str(),
                    title,
                    icon,
                    diff_idx == 0,
                );
            }
        });
}

fn report_file_header(parent: &mut HtmlElem, report_file: &ReportFile) {
    if let Some(old) = &report_file.left {
        parent.fieldset().class("control-group").with(|div| {
            div.a()
                .class("report-file-link")
                .title("Reference output")
                .href(display!("../../{}", old.path))
                .target("_blank")
                .with(|a| {
                    svg_icon(a, icons::LINK);
                    a.span().with(|span| {
                        span.text(display!("Reference"));
                        if old.size.is_none() {
                            span.span()
                                .class("file-size-change decreased")
                                .text("missing");
                        }
                    });
                });

            div.button()
                .class("icon-button copy-button")
                .title("Copy reference path")
                .data_attr("file-path", &old.path)
                .with(|button| {
                    svg_icon(button, icons::CHECK);
                    svg_icon(button, icons::COPY);
                })
        });
    }

    if let Some(new) = &report_file.right {
        parent.fieldset().class("control-group").with(|fieldset| {
            fieldset
                .a()
                .class("report-file-link")
                .title("Live output")
                .href(display!("../../{}", new.path))
                .target("_blank")
                .with(|a| {
                    svg_icon(a, icons::LINK);
                    if let Some(new_size) = new.size {
                        a.span().with(|span| {
                            span.text(display!("{}", FileSize(new_size)));

                            if let Some(old) = &report_file.left {
                                if let Some(old_size) = old.size {
                                    let size_change =
                                        new_size as isize - old_size as isize;
                                    let percent_change =
                                        typst_utils::round_with_precision(
                                            100.0 * size_change as f64 / old_size as f64,
                                            1,
                                        );
                                    let (symbol, class) = if size_change == 0 {
                                        ("", "")
                                    } else if percent_change == 0.0 {
                                        ("~", " slightly")
                                    } else if percent_change > 0.0 {
                                        ("+", " increased")
                                    } else {
                                        ("-", " decreased")
                                    };
                                    let abs_percent = percent_change.abs();
                                    span.span()
                                        .class(display!("file-size-change{class}"))
                                        .text(display!("{symbol}{abs_percent}%"));
                                }
                            } else {
                                span.span()
                                    .class("file-size-change increased")
                                    .text("added");
                            }
                        });
                    } else {
                        a.span().text("Missing");
                    }
                });

            fieldset
                .button()
                .class("icon-button copy-button")
                .title("Copy live path")
                .data_attr("file-path", &new.path)
                .with(|button| {
                    svg_icon(button, icons::CHECK);
                    svg_icon(button, icons::COPY);
                })
        });
    }
}

fn is_large_text_diff(diff: &FileDiff<Lines>) -> bool {
    let is_large = |lines: Option<&Lines>| {
        let Some(lines) = lines else { return false };

        if lines.lines.len() > 100 {
            return true;
        }

        let text_size = lines
            .lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.text.len()).sum::<usize>())
            .sum::<usize>();
        text_size > 1000
    };

    is_large(diff.left().and_then(|o| o.data()))
        || is_large(diff.right().and_then(|r| r.as_ref().ok()))
}

fn report_file_tab_panel(
    parent: &mut HtmlElem,
    report_file: &ReportFile,
    test_idx: usize,
    file_idx: usize,
) {
    parent
        .div()
        .aria_role("tabpanel")
        .aria_labelledby(display!("report-file-tab-{test_idx}-{}", report_file.output))
        .hidden(file_idx != 0)
        .class("report-file")
        .with(|div| {
            for (diff_idx, diff) in report_file.diffs.iter().enumerate() {
                file_diff_tabpanel(div, report_file, diff, test_idx, file_idx, diff_idx);
            }
        });
}

fn file_diff_tabpanel(
    parent: &mut HtmlElem,
    report_file: &ReportFile,
    diff: &DiffKind,
    test_idx: usize,
    file_idx: usize,
    diff_idx: usize,
) {
    let n = test_idx * TestOutput::ALL.len() + file_idx;
    parent
        .div()
        .aria_role("tabpanel")
        .aria_labelledby(display!("file-diff-tab-{n}-{}", diff.kind_str()))
        .hidden(diff_idx != 0)
        .class("file-diff")
        .with(|div| match diff {
            DiffKind::Text(diff) => text_diff(div, diff),
            DiffKind::Image(diff) => image_diff(div, report_file.output, diff, n),
        });
}

fn text_diff(parent: &mut HtmlElem, diff: &FileDiff<Lines>) {
    parent.table().class("text-diff").with(|table| {
        table.colgroup().with(|colgroup| {
            colgroup.col().attr("span", 1).class("col-line-gutter");
            colgroup.col().attr("span", 1).class("col-line-body");
            colgroup.col().attr("span", 1).class("col-line-gutter");
            colgroup.col().attr("span", 1).class("col-line-body");
        });

        let left = diff
            .left()
            .and_then(|old| old.data())
            .map(|l| l.lines.as_slice())
            .unwrap_or(&[]);
        let right = diff
            .right()
            .and_then(|res| res.as_ref().ok())
            .map(|l| l.lines.as_slice())
            .unwrap_or(&[]);

        text_diff_lines(table, left, right);

        table.tr().class("diff-line").with(|tr| {
            diff_line(tr, LineKind::End, 0, &[]);
            diff_line(tr, LineKind::End, 0, &[]);
        });
    });
}

fn text_diff_lines(table: &mut HtmlElem, left: &[Line], right: &[Line]) {
    let len = left.len().max(right.len());
    let empty_line = Line::EMPTY;
    for i in 0..len {
        // Fill missing lines.
        let l = left.get(i).unwrap_or(&empty_line);
        let r = right.get(i).unwrap_or(&empty_line);
        table.tr().class("diff-line").with(|tr| {
            diff_cells(tr, l);
            diff_cells(tr, r);
        });
    }
}

fn diff_cells(parent: &mut HtmlElem, line: &Line) {
    match line.kind {
        LineKind::Gap => {
            parent.td().colspan(2).class("diff-gap").text("\u{22EF}");
        }
        _ => {
            diff_line(parent, line.kind, line.nr, &line.spans);
        }
    }
}

fn diff_line(parent: &mut HtmlElem, kind: LineKind, line_nr: u32, spans: &[TextSpan]) {
    parent
        .td()
        .class(display!("line-gutter diff-{kind}"))
        .text_opt((line_nr != 0).then(|| display!("{line_nr}")));
    parent.td().class(display!("line-body diff-{kind}")).with(|td| {
        td.pre().class("line-text").with(|pre| {
            for span in spans {
                if span.emph && kind == LineKind::Del {
                    pre.del().text(&span.text);
                } else if span.emph && kind == LineKind::Add {
                    pre.ins().text(&span.text);
                } else {
                    pre.text(&span.text);
                }
            }
        });
    });
}

fn image_diff(
    parent: &mut HtmlElem,
    output: TestOutput,
    diff: &FileDiff<Image>,
    n: usize,
) {
    let radio_icon_button = |parent: &mut HtmlElem, name, value, title, icon, checked| {
        parent.label().class("icon-toggle-button").with(|label| {
            label
                .input()
                .type_("radio")
                .class(name)
                .name(display!("{name}-{n}"))
                .title(title)
                .value(value)
                .checked(checked);
            svg_icon(label, icon);
        });
    };

    let checkbox_icon_button = |parent: &mut HtmlElem, name, title, icon, checked| {
        parent.label().class("icon-toggle-button").with(|label| {
            label
                .input()
                .type_("checkbox")
                .class(name)
                .title(title)
                .checked(checked);
            svg_icon(label, icon);
        });
    };

    let icon_button = |parent: &mut HtmlElem, name, title, icon| {
        parent
            .button()
            .class(display!("icon-button {name}"))
            .title(title)
            .with(|button| svg_icon(button, icon));
    };

    struct SliderOpts {
        min: f32,
        max: f32,
        value: f32,
        step: f32,
    }
    impl std::ops::Mul<SliderOpts> for f32 {
        type Output = SliderOpts;

        fn mul(self, rhs: SliderOpts) -> Self::Output {
            SliderOpts {
                min: self * rhs.min,
                max: self * rhs.max,
                value: self * rhs.value,
                step: self * rhs.step,
            }
        }
    }
    impl Default for SliderOpts {
        fn default() -> Self {
            Self { min: 0.0, max: 1.0, value: 0.5, step: 0.01 }
        }
    }
    let slider = |parent: &mut HtmlElem, name, title, icon, opts: SliderOpts| {
        parent.label().class("slider").title(title).with(|label| {
            if let Some(icon) = icon {
                svg_icon(label, icon);
            }

            label
                .input()
                .type_("range")
                .class(name)
                .min(opts.min)
                .max(opts.max)
                .value(opts.value)
                .step(opts.step);
        })
    };

    parent.div().class("image-diff").with(|div| {
        div.div().class("image-controls").with(|div| {
            div.fieldset().class("control-group").with(|fieldset| {
                radio_icon_button(
                    fieldset,
                    "image-view-mode",
                    "side-by-side",
                    "View Mode side by side",
                    icons::VIEW_SIDE_BY_SIDE,
                    true,
                );
                radio_icon_button(
                    fieldset,
                    "image-view-mode",
                    "blend",
                    "View Mode blend",
                    icons::VIEW_BLEND,
                    false,
                );
                radio_icon_button(
                    fieldset,
                    "image-view-mode",
                    "difference",
                    "View Mode difference",
                    icons::VIEW_DIFFERENCE,
                    false,
                );
            });

            div.fieldset().class("control-group").with(|fieldset| {
                checkbox_icon_button(
                    fieldset,
                    "image-antialiasing",
                    "Antialiasing",
                    icons::ANTIALIASING,
                    true,
                );
            });

            div.fieldset().class("control-group").with(|fieldset| {
                icon_button(fieldset, "image-zoom-minus", "Zoom out", icons::MINUS);
                icon_button(fieldset, "image-zoom-plus", "Zoom in", icons::PLUS);

                // HACK: Scale factor of HTML pt (`1/72 inch`) to px (`1/96 inch`).
                // Since PNG images are rendered with 1 px/pt and PDFs converted
                // to SVGs don't currently specify a unit thus default to px.
                let factor = if output == TestOutput::Svg { 72.0 / 96.0 } else { 1.0 };
                slider(
                    fieldset,
                    "image-zoom",
                    "Zoom",
                    None,
                    factor * SliderOpts { min: 0.5, max: 8.0, value: 2.0, step: 0.05 },
                );
            });
        });

        div.div().class("image-diff-wrapper").with(|div| {
            let image = |parent: &mut HtmlElem<'_>, data_url: &str| {
                parent.img().src(data_url);
            };

            div.canvas().class("image-canvas").with(|canvas| {
                let data_url = (diff.left())
                    .and_then(|old| old.data())
                    .map(|img| img.data_url.as_str())
                    .unwrap_or("");
                image(canvas, data_url);

                let data_url = (diff.right())
                    .and_then(|res| res.as_ref().ok())
                    .map(|img| img.data_url.as_str())
                    .unwrap_or("");
                image(canvas, data_url);
            });
        });

        div.div().class("image-mode-controls").with(|div| {
            div.fieldset().class("control-group image-align-y-control").with(
                |fieldset| {
                    radio_icon_button(
                        fieldset,
                        "image-align-y",
                        "top",
                        "Vertical-align top",
                        icons::ALIGN_TOP,
                        true,
                    );
                    radio_icon_button(
                        fieldset,
                        "image-align-y",
                        "center",
                        "Vertical-align center",
                        icons::ALIGN_HORIZON,
                        false,
                    );
                    radio_icon_button(
                        fieldset,
                        "image-align-y",
                        "bottom",
                        "Vertical-align bottom",
                        icons::ALIGN_BOTTOM,
                        false,
                    );
                },
            );

            div.fieldset().class("control-group image-align-x-control").with(
                |fieldset| {
                    radio_icon_button(
                        fieldset,
                        "image-align-x",
                        "left",
                        "Horizontal-align left",
                        icons::ALIGN_LEFT,
                        true,
                    );
                    radio_icon_button(
                        fieldset,
                        "image-align-x",
                        "center",
                        "Horizontal-align center",
                        icons::ALIGN_CENTER,
                        false,
                    );
                    radio_icon_button(
                        fieldset,
                        "image-align-x",
                        "right",
                        "Horizontal-align right",
                        icons::ALIGN_RIGHT,
                        false,
                    );
                },
            );

            div.fieldset()
                .class("control-group image-blend-control")
                .with(|fieldset| {
                    slider(
                        fieldset,
                        "image-blend",
                        "Blend",
                        Some(icons::VIEW_BLEND),
                        SliderOpts { min: 0.0, max: 1.0, value: 0.5, step: 0.01 },
                    )
                });
        });
    });
}

#[rustfmt::skip]
mod icons {
    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub struct SvgIcon(&'static str);

    impl SvgIcon {
        pub fn as_str(&self) -> &'static str {
            self.0
        }
    }

    pub static RENDER: SvgIcon = SvgIcon("M1 3v10h14V3Zm1.2 1.2h11.6v5.2H9.567a.4.4 0 0 1-.343-.195L7.45 6.254a1.58 1.58 0 0 0-1.431-.756c-.553.021-1.096.313-1.372.863L2.2 11.26Zm9.3.8A1.5 1.5 0 0 0 10 6.5 1.5 1.5 0 0 0 11.5 8 1.5 1.5 0 0 0 13 6.5 1.5 1.5 0 0 0 11.5 5M6.064 6.656c.133-.005.268.066.358.215l1.771 2.951c.29.481.812.778 1.373.778h4.235v1.2H3.27l2.452-4.902c.077-.155.208-.237.341-.242");
    pub static PDF: SvgIcon = SvgIcon("M7.56 3.608 8 4.492l.442-.884c.05-.173.059-.373.011-.495a.2.2 0 0 0-.084-.106c-.045-.028-.15-.073-.369-.073-.218 0-.323.045-.369.073a.2.2 0 0 0-.084.106c-.047.122-.04.322.011.495m-.562-1.62c.28-.175.621-.254 1.003-.254s.722.08 1.003.254c.286.179.468.429.57.692.19.493.103 1.024-.004 1.343l-.013.04-.02.039-.855 1.711 2.68 4.644 1.905.115.043.002.042.009c.33.067.833.257 1.165.669.177.22.303.501.314.838.011.331-.09.666-.28.996-.192.331-.431.586-.723.742-.298.159-.605.19-.883.147-.523-.082-.94-.423-1.162-.674l-.029-.032-.023-.036-1.056-1.6H5.323L4.27 13.23l-.023.035-.028.032c-.224.252-.64.593-1.163.675a1.4 1.4 0 0 1-.883-.147c-.292-.156-.531-.411-.722-.742-.191-.33-.292-.665-.281-.996.011-.337.138-.62.314-.838.332-.412.836-.602 1.165-.67l.042-.008.043-.003 1.91-.114 2.677-4.64-.856-1.712-.019-.038-.013-.04c-.107-.32-.194-.85-.003-1.344.102-.263.283-.513.569-.692m5.163 9.72.987.059c.175.042.352.135.434.238a.2.2 0 0 1 .05.126c.002.053-.012.167-.121.355-.11.19-.201.258-.248.283a.2.2 0 0 1-.133.02c-.13-.02-.3-.127-.424-.257zm-8.322-.004-.545.825c-.124.13-.293.236-.423.257a.2.2 0 0 1-.134-.02c-.047-.025-.139-.094-.248-.283s-.122-.303-.12-.356a.2.2 0 0 1 .049-.126c.082-.102.26-.195.434-.238zm6.118-1.27L8 7.039l-1.965 3.395z");
    pub static PDFTAGS: SvgIcon = SvgIcon("m2.732.732.084.747.051.458 1.227.168-.002-.013 3.287.365a3.4 3.4 0 0 1 2.03.975l4.01 4.011-.32.32.85.848.744-.744.424-.424-.424-.423-4.435-4.436a4.6 4.6 0 0 0-2.746-1.32L3.479.816Zm-2 2 .084.747.448 4.033a4.6 4.6 0 0 0 1.32 2.746l4.436 4.435.423.424.424-.424 4.826-4.826.424-.424-.424-.423-4.435-4.436a4.6 4.6 0 0 0-2.746-1.32l-4.033-.448Zm1.36 1.36 3.287.365a3.4 3.4 0 0 1 2.03.975l4.01 4.011-3.976 3.977-4.011-4.012a3.4 3.4 0 0 1-.975-2.03ZM5 6a1 1 0 1 0 0 2 1 1 0 0 0 0-2");
    pub static SVG: SvgIcon = SvgIcon("M2.93 12v2h2v-2zm0-10v2h2V2Zm.46 2v8h1.2V4Zm9.68 0c-1.46 0-3.741-.01-5.726.438-.993.223-1.922.557-2.647 1.12l-.107.088v4.708q.052.046.107.087c.725.564 1.654.898 2.647 1.121 1.985.447 4.266.438 5.726.438v-1.2c-1.461 0-3.678-.006-5.463-.407-.892-.201-1.666-.506-2.171-.899-.506-.393-.764-.82-.764-1.494 0-.673.258-1.101.764-1.494.505-.393 1.28-.698 2.171-.899C9.392 5.206 11.61 5.2 13.07 5.2Z");
    pub static HTML: SvgIcon = SvgIcon("M8 1.4A6.61 6.61 0 0 0 1.4 8c0 3.638 2.962 6.6 6.6 6.6s6.6-2.962 6.6-6.6S11.638 1.4 8 1.4m1.824.883c-.106.149-.2.31-.285.47-.166.316-.281.63-.281.84 0 .287.21.594.37.772a.3.3 0 0 0 .224.094.33.33 0 0 0 .332-.332v-.1c0-.182.086-.373.189-.533a.35.35 0 0 1 .37-.15q.137.08.27.17l.003.002a.46.46 0 0 1 .093.279v.121c0 .323-.21.609-.52.705l-.868.272-.496.23c-.286.133-.565.302-.729.57-.093.152-.166.328-.166.496 0 .433.465.866.928.866.2 0 .409-.17.584-.371.253-.291.548-.56.916-.674.504-.157 1.145-.052 1.262.463q.016.075.017.148c0 .216-.117.217-.232.217-.116 0-.23 0-.23.217 0 .456.668.285 1.04.021.193-.137.348-.282.348-.455 0-.12.105-.199.209-.187a5.4 5.4 0 0 1 .201 2.119 2.7 2.7 0 0 0-1.146-.475c-.563-.089-1.133-.158-1.53-.158-.26 0-.553-.03-.845-.059a8 8 0 0 0-.942-.054c-.472.014-.816.142-.816.572 0 .325-.085.695-.166 1.043-.148.634-.279 1.193.166 1.252 1.216.161 1.96 1.033 2.31 2.164-.723.36-1.54.562-2.404.562a5.4 5.4 0 0 1-2.084-.414l-.014.035q.029-.079.05-.162c.04-.171.075-.3.1-.367.065-.164.208-.297.376-.453.237-.22.524-.486.705-.95.15-.383-.487-.76-1.229-1.048a2.1 2.1 0 0 0-.818-.146c-.598.015-1.157.297-1.47.843q-.07.121-.128.235a5.4 5.4 0 0 1-.767-1.825l.736.49A.35.35 0 0 0 4 9.349c0-.193.159-.341.34-.276.194.07.432.2.66.428q.121.119.244.17c.417.18.495-.431.174-.752a.77.77 0 0 1-.133-.938c.203-.34.466-.73.715-.98.32-.32-.195-1.092-.67-1.643a2.4 2.4 0 0 0-.766-.574l-.222-.113a3 3 0 0 0-.45-.182A5.4 5.4 0 0 1 5 3.506C5.005 4.29 5.15 5 5.627 5c.206 0 .395-.023.564-.059.636-.134.84-.827.682-1.457l-.125-.494a1 1 0 0 0-.088-.224A5.4 5.4 0 0 1 8 2.6c.522 0 1.027.073 1.504.21.01-.019.025-.037.035-.056.085-.161.178-.322.285-.47M8.415 3.967a.25.25 0 0 0-.233.32l.138.47a.418.418 0 1 0 .645-.456l-.4-.285a.25.25 0 0 0-.15-.05m2.503 2.756a.4.4 0 0 0-.145.039c-.36.18-.233.724.17.724a.383.383 0 1 0-.025-.764m-5.102 6.502-.035.066z");

    pub static IMAGE: SvgIcon = SvgIcon("M1 3v10h14V3Zm1.2 1.2h11.6v5.2H9.567a.4.4 0 0 1-.343-.195L7.45 6.254a1.58 1.58 0 0 0-1.431-.756c-.553.021-1.096.313-1.372.863L2.2 11.26Zm9.3.8A1.5 1.5 0 0 0 10 6.5 1.5 1.5 0 0 0 11.5 8 1.5 1.5 0 0 0 13 6.5 1.5 1.5 0 0 0 11.5 5M6.064 6.656c.133-.005.268.066.358.215l1.771 2.951c.29.481.812.778 1.373.778h4.235v1.2H3.27l2.452-4.902c.077-.155.208-.237.341-.242");
    pub static TEXT: SvgIcon = SvgIcon("M11.291 5.555a2 2 0 0 0-1.117.316q-.493.314-.776.904-.279.591-.279 1.418 0 .826.283 1.393.287.564.776.853.493.288 1.107.288.475 0 .772-.149a1.4 1.4 0 0 0 .466-.351 2.3 2.3 0 0 0 .262-.368h.065v1.02q0 .607-.373.883-.375.28-.95.281-.418 0-.683-.12a1.3 1.3 0 0 1-.42-.286 2 2 0 0 1-.242-.313l-.868.358q.14.316.418.582.28.266.723.43.445.16 1.063.161.66 0 1.187-.207.53-.205.84-.632.31-.43.31-1.098V5.621h-.988v.842h-.074a3 3 0 0 0-.26-.375 1.4 1.4 0 0 0-.463-.371q-.297-.162-.779-.162m.217.857q.441 0 .742.225.3.222.455.62.155.4.154.923 0 .535-.158.92-.154.381-.459.588-.3.2-.734.2-.45.001-.756-.214a1.33 1.33 0 0 1-.459-.602 2.4 2.4 0 0 1-.154-.892q0-.496.15-.899a1.4 1.4 0 0 1 .459-.633q.304-.236.76-.236M4.39 3.895l-2.487 6.91h1.108l.633-1.83h2.697l.633 1.83h1.107l-2.486-6.91Zm.578 1.255h.052L6.04 8.098H3.95Z");

    pub static COPY: SvgIcon = SvgIcon("M6.727 2c-.545 0-1 .455-1 1v7c0 .545.455 1 1 1H12c.545 0 1-.455 1-1V3c0-.545-.455-1-1-1Zm.25 1.25h4.773v6.5H6.977ZM4 5c-.545 0-1 .455-1 1v7c0 .545.455 1 1 1h5.273c.545 0 1-.455 1-1v-1h-1.25v.75H4.25v-6.5h.568V5Z");
    pub static LINK: SvgIcon = SvgIcon("M7.81 3.374a3.403 3.403 0 0 1 4.816 0 3.403 3.403 0 0 1 0 4.817l-1.255 1.255-.884-.884 1.255-1.255a2.153 2.153 0 0 0 0-3.05 2.153 2.153 0 0 0-3.049 0L7.438 5.514l-.884-.884zM5.632 9.483l3.85-3.85.884.884-3.85 3.85zM4.63 6.553 3.374 7.81a3.403 3.403 0 0 0 0 4.817 3.403 3.403 0 0 0 4.817 0l1.255-1.255-.884-.884-1.255 1.255a2.153 2.153 0 0 1-3.05 0 2.153 2.153 0 0 1 0-3.049l1.256-1.255z");
    pub static CHECK: SvgIcon = SvgIcon("m13.094 4.094-6.63 6.629-3.093-3.094-.885.885 3.537 3.535.442.441.441-.441 7.07-7.072Z");
    pub static SEARCH: SvgIcon = SvgIcon("M6.781 2a4.781 4.781 0 1 0 2.91 8.576l3.422 3.42.883-.883-3.42-3.422A4.781 4.781 0 0 0 6.781 2m0 1.25a3.531 3.531 0 1 1 0 7.062 3.531 3.531 0 0 1 0-7.062");

    pub static VIEW_SIDE_BY_SIDE: SvgIcon = SvgIcon("M7.43 5.438v6.568h1.25V5.438ZM3.744 3.047a1 1 0 0 0-1 1v7.92a1 1 0 0 0 1 1h8.512a1 1 0 0 0 1-1v-7.92a1 1 0 0 0-1-1H3.869zm.942.857a.685.685 0 1 1 0 1.371.685.685 0 0 1 0-1.37M7.43 5.438h1.25v.609h3.326v5.67H8.68v.289H7.43v-.29H3.994v-5.67H7.43z");
    pub static VIEW_BLEND: SvgIcon = SvgIcon("M4.086 8.12 1.158 9.183a.2.2 0 0 0-.025.365l6.805 3.617c.25.133.544.153.81.057l5.533-2.012a.199.199 0 0 0 .026-.363l-2.952-1.57-2.503.91a1.25 1.25 0 0 1-1.014-.073zm3.203-5.403a1 1 0 0 0-.307.06L1.027 4.941l7.2 3.83a1 1 0 0 0 .812.057l5.953-2.166-7.199-3.83a1 1 0 0 0-.504-.115m.02 1.217 4.744 2.523-3.34 1.215L3.97 5.148Z");
    pub static VIEW_DIFFERENCE: SvgIcon  = SvgIcon("M5 4a4 4 0 1 0 1.693 7.625c.14-.065.142-.258.016-.346A4 4 0 0 1 5 8c0-1.357.676-2.556 1.709-3.28.126-.088.124-.28-.016-.345A4 4 0 0 0 5 4m6 0a4.01 4.01 0 0 0-4 4c0 2.202 1.798 4 4 4s4-1.798 4-4-1.798-4-4-4m0 1.2c1.554 0 2.8 1.246 2.8 2.8s-1.246 2.8-2.8 2.8A2.79 2.79 0 0 1 8.2 8c0-1.554 1.246-2.8 2.8-2.8");

    pub static ALIGN_TOP: SvgIcon = SvgIcon("M2 3.016v1.25h12v-1.25zm6 3-.443.441-2 2-.442.441.885.885.441-.441.934-.934v4.582h1.25V8.408l.932.934.443.441.883-.885-.442-.441-2-2Z");
    pub static ALIGN_HORIZON: SvgIcon = SvgIcon("M7.375 10.174v4.857h1.25v-4.857zM8 9.504l-2.441 2.441-.444.444.885.882.441-.44.934-.935v-1.722h1.25v1.722l.934.934.441.441.883-.882-.442-.444Zm-.625-8.486v3.134l-.934-.931L6 2.779l-.885.883.444.443 2 2L8 6.547l.441-.442 2.002-2 .442-.443L10 2.78l-.441.442-.934.931V1.018ZM2 7.75V9h12V7.75Z");
    pub static ALIGN_BOTTOM: SvgIcon = SvgIcon("M2 11.75V13h12v-1.25Zm5.375-8.775V7.64l-.932-.934L6 6.266l-.883.884.442.442 2 2 .441.441.443-.441 2-2 .442-.442L10 6.266l-.441.441-.934.934V2.975Z");

    pub static ALIGN_LEFT: SvgIcon = SvgIcon("M3.38 2v12h1.25V2ZM2 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V9Zm2.63 0v3H6c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM2 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V4Zm2.63 0v3H12c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");
    pub static ALIGN_CENTER: SvgIcon = SvgIcon("M7.375 2v12h1.25V2ZM6 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H10c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM3 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h4.375V4Zm5.625 0v3H13c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");
    pub static ALIGN_RIGHT: SvgIcon = SvgIcon("M11.375 2v12h1.25V2ZM10 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H14c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM4 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h7.375V4Zm8.625 0v3H14c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");

    pub static ANTIALIASING: SvgIcon = SvgIcon("M4 11v3h3v-3ZM2 8v3h3V8Zm0-3v3h3V5Zm2-3v3h3V2Zm5 .084v1.273a4.75 4.75 0 0 1 0 9.286v1.271a5.999 5.999 0 0 0 0-11.83M7.375 1v14h1.25V1Z");

    pub static PLUS: SvgIcon = SvgIcon("M7.375 2v5.375H2v1.25h5.375V14h1.25V8.625H14v-1.25H8.625V2Z");
    pub static MINUS: SvgIcon = SvgIcon("M2 7.375v1.25h12v-1.25z");

    pub static CHEVRON_DOWN: SvgIcon = SvgIcon("m3.441 5.338-.882.883 5 5 .441.441.441-.441 5-5-.882-.883L8 9.895Z");
}
