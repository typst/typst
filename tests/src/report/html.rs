use std::collections::hash_map::Entry;
use std::fmt::Display;

use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;
use xmlwriter::XmlWriter;

use crate::report::html::icons::SvgIcon;
use crate::report::{DiffKind, ImageFileDiff, Line, TestReport, TextFileDiff, TextSpan};

use super::LineKind;

#[rustfmt::skip]
mod icons {
    #[derive(Copy, Clone, Eq, PartialEq, Hash)]
    pub struct SvgIcon(&'static str);

    impl SvgIcon {
        pub fn as_str(&self) -> &'static str {
            self.0
        }
    }

    pub static VIEW_SIDE_BY_SIDE: SvgIcon = SvgIcon("M7.43 5.438v6.568h1.25V5.438ZM3.744 3.047a1 1 0 0 0-1 1v7.92a1 1 0 0 0 1 1h8.512a1 1 0 0 0 1-1v-7.92a1 1 0 0 0-1-1H3.869zm.942.857a.685.685 0 1 1 0 1.371.685.685 0 0 1 0-1.37M7.43 5.438h1.25v.609h3.326v5.67H8.68v.289H7.43v-.29H3.994v-5.67H7.43z");
    pub static VIEW_BLEND: SvgIcon = SvgIcon("M4.086 8.12 1.158 9.183a.2.2 0 0 0-.025.365l6.805 3.617c.25.133.544.153.81.057l5.533-2.012a.199.199 0 0 0 .026-.363l-2.952-1.57-2.503.91a1.25 1.25 0 0 1-1.014-.073zm3.203-5.403a1 1 0 0 0-.307.06L1.027 4.941l7.2 3.83a1 1 0 0 0 .812.057l5.953-2.166-7.199-3.83a1 1 0 0 0-.504-.115m.02 1.217 4.744 2.523-3.34 1.215L3.97 5.148Z");
    pub static VIEW_DIFFERENCE: SvgIcon  = SvgIcon("M5 4a4 4 0 1 0 1.693 7.625c.14-.065.142-.258.016-.346A4 4 0 0 1 5 8c0-1.357.676-2.556 1.709-3.28.126-.088.124-.28-.016-.345A4 4 0 0 0 5 4m6 0a4.01 4.01 0 0 0-4 4c0 2.202 1.798 4 4 4s4-1.798 4-4-1.798-4-4-4m0 1.2c1.554 0 2.8 1.246 2.8 2.8s-1.246 2.8-2.8 2.8A2.79 2.79 0 0 1 8.2 8c0-1.554 1.246-2.8 2.8-2.8");

    pub static ALIGN_TOP: SvgIcon = SvgIcon("M2 3.016v1.25h12v-1.25zm6 3-.443.441-2 2-.442.441.885.885.441-.441.934-.934v4.582h1.25V8.408l.932.934.443.441.883-.885-.442-.441-2-2Z");
    pub const ALIGN_HORIZON: SvgIcon = SvgIcon("M7.375 10.174v4.857h1.25v-4.857zM8 9.504l-2.441 2.441-.444.444.885.882.441-.44.934-.935v-1.722h1.25v1.722l.934.934.441.441.883-.882-.442-.444Zm-.625-8.486v3.134l-.934-.931L6 2.779l-.885.883.444.443 2 2L8 6.547l.441-.442 2.002-2 .442-.443L10 2.78l-.441.442-.934.931V1.018ZM2 7.75V9h12V7.75Z");
    pub static ALIGN_BOTTOM: SvgIcon = SvgIcon("M2 11.75V13h12v-1.25Zm5.375-8.775V7.64l-.932-.934L6 6.266l-.883.884.442.442 2 2 .441.441.443-.441 2-2 .442-.442L10 6.266l-.441.441-.934.934V2.975Z");

    pub static ALIGN_LEFT: SvgIcon = SvgIcon("M3.38 2v12h1.25V2ZM2 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V9Zm2.63 0v3H6c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM2 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.38V4Zm2.63 0v3H12c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");
    pub static ALIGN_CENTER: SvgIcon = SvgIcon("M7.375 2v12h1.25V2ZM6 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H10c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM3 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h4.375V4Zm5.625 0v3H13c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");
    pub static ALIGN_RIGHT: SvgIcon = SvgIcon("M11.375 2v12h1.25V2ZM10 9c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h1.375V9Zm2.625 0v3H14c.554 0 1-.446 1-1v-1c0-.554-.446-1-1-1ZM4 4c-.554 0-1 .446-1 1v1c0 .554.446 1 1 1h7.375V4Zm8.625 0v3H14c.554 0 1-.446 1-1V5c0-.554-.446-1-1-1Z");

    pub static ANTIALIASING: SvgIcon = SvgIcon("M4 11v3h3v-3ZM2 8v3h3V8Zm0-3v3h3V5Zm2-3v3h3V2Zm5 .084v1.273a4.75 4.75 0 0 1 0 9.286v1.271a5.999 5.999 0 0 0 0-11.83M7.375 1v14h1.25V1Z");

    pub static PLUS: SvgIcon = SvgIcon("M7.375 2v5.375H2v1.25h5.375V14h1.25V8.625H14v-1.25H8.625V2Z");
    pub static MINUS: SvgIcon = SvgIcon("M2 7.375v1.25h12v-1.25z");
}

static REPORT_STYLE: &str = include_str!("report.css");
static REPORT_SCRIPT: &str = include_str!("report.js");

macro_rules! display {
    ($($arg:tt)*) => {
        ::typst_utils::display(|f| write!(f, $($arg)*))
    }
}

struct Html {
    /// A map from the svg icon path to a cached id.
    svg_icon_cache: FxHashMap<SvgIcon, EcoString>,
    writer: XmlWriter,
}

impl Html {
    fn new() -> Self {
        Self {
            svg_icon_cache: FxHashMap::default(),
            writer: XmlWriter::new(xmlwriter::Options {
                use_single_quote: false,
                indent: xmlwriter::Indent::None,
                attributes_indent: xmlwriter::Indent::None,
            }),
        }
    }

    fn finish(self) -> String {
        self.writer.end_document()
    }

    fn elem(&mut self, name: &str) -> HtmlElem<'_> {
        HtmlElem::new(self, name)
    }
}

struct HtmlElem<'a> {
    html: &'a mut Html,
}

impl<'a> HtmlElem<'a> {
    fn new(html: &'a mut Html, name: &str) -> Self {
        html.writer.start_element(name);
        Self { html }
    }

    fn elem(&mut self, name: &str) -> HtmlElem<'_> {
        HtmlElem::new(self.html, name)
    }

    fn attr(&mut self, name: &str, val: impl Display) -> &mut Self {
        self.html.writer.write_attribute(name, &val);
        self
    }

    fn opt_attr(&mut self, name: &str, val: Option<impl Display>) -> &mut Self {
        if let Some(val) = val {
            self.html.writer.write_attribute(name, &val);
        }
        self
    }

    fn text(&mut self, text: &str) -> &mut Self {
        self.html.writer.write_text(text);
        self
    }

    fn text_opt(&mut self, text: Option<impl Display>) -> &mut Self {
        if let Some(text) = text {
            self.html.writer.write_text_fmt(format_args!("{text}"));
        }
        self
    }

    fn with<'e, T>(&'e mut self, f: impl FnOnce(&'e mut Self) -> T) -> T {
        f(self)
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

macro_rules! attr_methods {
    ($(fn $name:ident($val_ty:ty);)+) => {
        $(
            fn $name(&mut self, $name: $val_ty) -> &mut Self {
                self.attr(stringify!($name), $name)
            }
        )+
    };
}

/// Convenience methods.
impl HtmlElem<'_> {
    elem_methods! {
        fn h1();
        fn h2();
        fn div();
        fn fieldset();
        fn label();
        fn input();
        fn button();
        fn a();
        fn img();

        fn ul();
        fn li();

        fn details();
        fn summary();

        fn table();
        fn colgroup();
        fn col();
        fn tr();
        fn td();

        fn pre();
        fn del();
        fn ins();
    }

    attr_methods! {
        fn id(&str);
        fn name(impl Display);
        fn class(impl Display);
        fn title(&str);

        fn href(impl Display);

        fn src(impl Display);

        fn value(impl Display);
        fn min(f32);
        fn max(f32);
        fn step(f32);

        fn span(u32);
        fn colspan(u32);
    }

    fn type_(&mut self, ty: &str) -> &mut Self {
        self.attr("type", ty);
        self
    }

    fn checked(&mut self, checked: bool) -> &mut Self {
        self.opt_attr("checked", checked.then_some("checked"));
        self
    }

    fn open(&mut self, open: bool) -> &mut Self {
        self.opt_attr("open", open.then_some("open"));
        self
    }
}

impl Drop for HtmlElem<'_> {
    fn drop(&mut self) {
        self.html.writer.end_element();
    }
}

pub fn generate(mut reports: Vec<TestReport>) -> String {
    reports.sort_by(|a, b| a.name.cmp(&b.name));

    let mut html = Html::new();

    html.elem("html").attr("lang", "en").with(|root| {
        root.elem("head").with(|head| {
            head.elem("meta").attr("charset", "utf-8");
            head.elem("title").text("Typst test report");
            head.elem("style").text(REPORT_STYLE);
        });

        root.elem("body").with(|body| {
            write_reports(body, reports);

            body.elem("script").text(REPORT_SCRIPT);
        });
    });

    html.finish()
}

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
                svg.elem("use").attr("href", display!("#{id}"));
            }
            Entry::Vacant(vacant) => {
                let id = eco_format!("svg-icon-{n}");
                vacant.insert(id.clone());
                svg.elem("path").id(&id).attr("d", icon.as_str());
            }
        });
}

fn write_reports(body: &mut HtmlElem, reports: Vec<TestReport>) {
    body.div().class("container").with(|div| {
        div.div().class("sidebar-container").with(|div| {
            div.div().class("sidebar").with(|div| {
                // TODO: Fix list structure.
                div.ul().class("sidebar-list").with(|ul| {
                    ul.h2().text("Settings");
                    ul.fieldset().class("control-group").with(|fieldset| {
                        // TODO: Maybe add macro to implement this as a trait extension?
                        let icon_button = |parent: &mut HtmlElem, id, title, icon| {
                            parent
                                .button()
                                .class("icon-button")
                                .id(id)
                                .title(title)
                                .with(|button| svg_icon(button, icon));
                        };

                        icon_button(
                            fieldset,
                            "global-image-view-mode-side-by-side",
                            "Global Image View-Mode side by side",
                            icons::VIEW_SIDE_BY_SIDE,
                        );
                        icon_button(
                            fieldset,
                            "global-image-view-mode-blend",
                            "Global Image View-Mode blend",
                            icons::VIEW_BLEND,
                        );
                        icon_button(
                            fieldset,
                            "global-image-view-mode-difference",
                            "Global Image View-Mode difference",
                            icons::VIEW_DIFFERENCE,
                        );
                    });

                    ul.h2().text("Failed tests");
                    for report in reports.iter() {
                        ul.li().with(|li| {
                            li.a().href(display!("#{}", report.name)).text(&report.name);
                        });
                    }
                    if reports.is_empty() {
                        ul.div().class("sidebar-empty").text("NONE");
                    }
                });
            });
        });

        div.div().class("diff-container").with(|div| {
            div.h2().class("diff-container-header").text("Changes");

            let mut num_image_diffs = 0;
            for report in reports.iter() {
                for (i, diff) in report.diffs.iter().enumerate() {
                    let file_diff_id = (i == 0).then_some(&report.name);
                    let close_by_default = match diff {
                        DiffKind::Text(diff) => {
                            let sum_text_len = |lines: &[Line]| {
                                lines
                                    .iter()
                                    .map(|l| {
                                        l.spans
                                            .iter()
                                            .map(|s| s.text.len())
                                            .sum::<usize>()
                                    })
                                    .sum::<usize>()
                            };
                            diff.left.lines.len() > 100
                                || sum_text_len(&diff.left.lines) > 1000
                                || sum_text_len(&diff.right.lines) > 1000
                        }
                        DiffKind::Image(_) => false,
                    };
                    div.div().class("file-diff").opt_attr("id", file_diff_id).with(
                        |div| {
                            div.details().open(!close_by_default).with(|details| {
                                details.summary().class("diff-summary").with(|summary| {
                                    summary.h1().class("diff-header").with(|h1| {
                                        h1.div().class("diff-header-split").with(|div| {
                                            div.a()
                                                .href(display!(
                                                    "../../{}",
                                                    diff.left_path()
                                                ))
                                                .text(diff.left_path());
                                        });
                                        h1.div().class("diff-header-split").with(|div| {
                                            div.a()
                                                .href(display!(
                                                    "../../{}",
                                                    diff.right_path()
                                                ))
                                                .text(diff.right_path());
                                        });
                                    });
                                    summary.div().class("diff-spacer");
                                });

                                match diff {
                                    DiffKind::Text(diff) => {
                                        text_diff(details, diff);
                                    }
                                    DiffKind::Image(diff) => {
                                        image_diff(details, diff, num_image_diffs);
                                        num_image_diffs += 1;
                                    }
                                }
                            });
                        },
                    );
                }
            }

            if reports.is_empty() {
                div.div().class("diff-container-empty").text("NONE");
            }

            div.div().class("diff-scroll-padding");
        });
    })
}

fn text_diff(parent: &mut HtmlElem, diff: &TextFileDiff) {
    parent.table().class("text-diff").with(|table| {
        table.colgroup().with(|colgroup| {
            colgroup.col().span(1).class("col-line-gutter");
            colgroup.col().span(1).class("col-line-body");
            colgroup.col().span(1).class("col-line-gutter");
            colgroup.col().span(1).class("col-line-body");
        });

        for (l, r) in diff.left.lines.iter().zip(diff.right.lines.iter()) {
            table.tr().class("diff-line").with(|tr| {
                diff_cells(tr, l);
                diff_cells(tr, r);
            });
        }
        table.tr().class("diff-line").with(|tr| {
            diff_line(tr, LineKind::End, 0, &[]);
            diff_line(tr, LineKind::End, 0, &[]);
        });
    });
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

fn image_diff(parent: &mut HtmlElem, diff: &ImageFileDiff, n: usize) {
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
    impl Default for SliderOpts {
        fn default() -> Self {
            Self { min: 0.0, max: 1.0, value: 0.5, step: 0.01 }
        }
    }
    let slider = |parent: &mut HtmlElem, name, title, icon, opts: SliderOpts| {
        parent.label().class("slider").title(title).with(|label| {
            if let Some(icon) = icon {
                svg_icon(label, icon)
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
                    "View-Mode side by side",
                    icons::VIEW_SIDE_BY_SIDE,
                    true,
                );
                radio_icon_button(
                    fieldset,
                    "image-view-mode",
                    "blend",
                    "View-Mode blend",
                    icons::VIEW_BLEND,
                    false,
                );
                radio_icon_button(
                    fieldset,
                    "image-view-mode",
                    "difference",
                    "View-Mode difference",
                    icons::VIEW_DIFFERENCE,
                    false,
                );
            });

            div.fieldset().class("control-group").with(|fieldset| {
                checkbox_icon_button(
                    fieldset,
                    "antialiasing",
                    "Antialiasing",
                    icons::ANTIALIASING,
                    true,
                );
            });

            div.fieldset().class("control-group").with(|fieldset| {
                icon_button(fieldset, "image-zoom-minus", "Zoom out", icons::MINUS);
                icon_button(fieldset, "image-zoom-plus", "Zoom in", icons::PLUS);
                slider(
                    fieldset,
                    "image-zoom",
                    "Zoom",
                    None,
                    SliderOpts { min: 0.5, max: 8.0, value: 2.0, step: 0.05 },
                );
            });
        });

        div.div().class("image-diff-area").with(|div| {
            div.div().class("image-diff-wrapper").with(|div| {
                div.div().class("image-split side-by-side").with(|div| {
                    div.img().src(&diff.left.data_url);
                });
                div.div().class("image-split side-by-side").with(|div| {
                    div.img().src(&diff.right.data_url);
                });
            });
        });

        div.div().class("image-mode-controls").with(|div| {
            div.fieldset().class("control-group").with(|fieldset| {
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
            });

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
