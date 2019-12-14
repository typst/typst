//! Styles for text and pages.

use toddle::query::FontClass;
use FontClass::*;

use crate::size::{Size, Size2D, SizeBox};
use crate::syntax::ParseResult;

/// Defines properties of pages and text.
#[derive(Debug, Default, Clone)]
pub struct LayoutStyle {
    pub page: PageStyle,
    pub text: TextStyle,
}

/// Defines which fonts to use and how to space text.
#[derive(Debug, Clone)]
pub struct TextStyle {
    /// The classes the font has to be part of.
    pub classes: Vec<FontClass>,
    /// The fallback classes from which the font needs to match the
    /// leftmost possible one.
    pub fallback: Vec<FontClass>,
    /// The base font size.
    pub base_font_size: Size,
    /// The font scale to apply on the base font size.
    pub font_scale: f32,
    /// The word spacing (as a multiple of the font size).
    pub word_spacing: f32,
    /// The line spacing (as a multiple of the font size).
    pub line_spacing: f32,
    /// The paragraphs spacing (as a multiple of the font size).
    pub paragraph_spacing: f32,
}

impl TextStyle {
    /// The scaled font size.
    pub fn font_size(&self) -> Size {
        self.base_font_size * self.font_scale
    }

    /// Toggle a class.
    ///
    /// If the class was one of _italic_ or _bold_, then:
    /// - If it was not present before, the _regular_ class will be removed.
    /// - If it was present before, the _regular_ class will be added in case the other
    ///   style class is not present.
    pub fn toggle_class(&mut self, class: FontClass) {
        if self.classes.contains(&class) {
            // If we retain a Bold or Italic class, we will not add
            // the Regular class.
            let mut regular = true;
            self.classes.retain(|x| {
                if class == *x {
                    false
                } else {
                    if class == Bold || class == Italic {
                        regular = false;
                    }
                    true
                }
            });

            if regular {
                self.classes.push(Regular);
            }
        } else {
            // If we add an Italic or Bold class, we remove
            // the Regular class.
            if class == Italic || class == Bold {
                self.classes.retain(|x| x != &Regular);
            }

            self.classes.push(class);
        }
    }
}

impl Default for TextStyle {
    fn default() -> TextStyle {
        TextStyle {
            classes: vec![Regular],
            fallback: vec![Serif],
            base_font_size: Size::pt(11.0),
            font_scale: 1.0,
            word_spacing: 0.25,
            line_spacing: 1.2,
            paragraph_spacing: 1.5,
        }
    }
}

/// Defines the size and margins of a page.
#[derive(Debug, Copy, Clone)]
pub struct PageStyle {
    /// The width and height of the page.
    pub dimensions: Size2D,
    /// The amount of white space on each side.
    pub margins: SizeBox,
}

impl Default for PageStyle {
    fn default() -> PageStyle {
        PageStyle {
            // A4 paper.
            dimensions: Size2D {
                x: Size::mm(210.0),
                y: Size::mm(297.0),
            },

            // All the same margins.
            margins: SizeBox {
                left: Size::cm(2.5),
                top: Size::cm(2.5),
                right: Size::cm(2.5),
                bottom: Size::cm(2.5),
            },
        }
    }
}

macro_rules! papers {
    ($(($var:ident: $width:expr, $height: expr, $($patterns:tt)*))*) => {
        $(/// The size of the paper that's in the name.
        pub const $var: Size2D = Size2D {
            x: Size { points: 2.83465 * $width },
            y: Size { points: 2.83465 * $height },
        };)*

        /// The size of a page with the given name.
        pub fn parse_paper_name(paper: &str) -> ParseResult<Size2D> {
            Ok(match paper.to_lowercase().as_str() {
                $($($patterns)* => $var,)*
                _ => error!("unknown paper size: `{}`", paper),
            })
        }
    };
}

// All paper sizes in mm.
papers! {
    // ---------------------------------------------------------------------- //
    // ISO 216

    // A Series
    (PAPER_A0:  841.0, 1189.0, "a0" | "poster")
    (PAPER_A1:  594.0, 841.0,  "a1")
    (PAPER_A2:  420.0, 594.0,  "a2")
    (PAPER_A3:  297.0, 420.0,  "a3")
    (PAPER_A4:  210.0, 297.0,  "a4")
    (PAPER_A5:  148.0, 210.0,  "a5")
    (PAPER_A6:  105.0, 148.0,  "a6")
    (PAPER_A7:  74.0,  105.0,  "a7" | "iso-7810-id-2" | "id-2" | "visa" | "flyer")
    (PAPER_A8:  52.0,  74.0,   "a8")
    (PAPER_A9:  37.0,  52.0,   "a9")
    (PAPER_A10: 26.0,  37.0,   "a10")
    (PAPER_A11: 18.0,  26.0,   "a11")

    // B Series
    (PAPER_B1: 707.0, 1000.0, "b1" | "flipchart")
    (PAPER_B2: 500.0, 707.0,  "b2")
    (PAPER_B3: 353.0, 500.0,  "b3")
    (PAPER_B4: 250.0, 353.0,  "b4" | "sheet-music")
    (PAPER_B5: 176.0, 250.0,  "b5" | "book")
    (PAPER_B6: 125.0, 176.0,  "b6")
    (PAPER_B7: 88.0,  125.0,  "b7" | "passport" | "iso-7810-id-3" | "id-3")
    (PAPER_B8: 62.0,  88.0,   "b8")

    // C Series
    (PAPER_C3: 324.0, 458.0, "c3")
    (PAPER_C4: 229.0, 324.0, "c4")
    (PAPER_C5: 162.0, 229.0, "c5")
    (PAPER_C6: 114.0, 162.0, "c6")
    (PAPER_C7: 81.0, 114.0,  "c7")
    (PAPER_C8: 57.0, 81.0,   "c8")

    // D Series (DIN extension to ISO)
    (PAPER_D3: 272.0, 385.0, "din-d3")
    (PAPER_D4: 192.0, 272.0, "din-d4")
    (PAPER_D5: 136.0, 192.0, "din-d5" | "dvd")
    (PAPER_D6: 96.0, 136.0,  "din-d6")
    (PAPER_D7: 68.0,  96.0,  "din-d7")
    (PAPER_D8: 48.0,  68.0,  "din-d8")

    // Academically relevant SIS extensions
    (PAPER_G5: 169.0, 239.0, "sis-g5")
    (PAPER_E5: 115.0, 220.0, "sis-e5")

    // ---------------------------------------------------------------------- //
    // Unites States

    // Customary
    (PAPER_FOLIO:             210.0, 330.0, "folio" | "us-folio" | "us-f4")
    (PAPER_LETTER:            216.0, 279.0, "letter" | "ansi-a" | "american-quarto" | "carta")
    (PAPER_LEGAL:             216.0, 356.0, "legal")
    (PAPER_TABLOID:           279.0, 432.0, "tabloid" | "ansi-b")
    (PAPER_LEDGER:            432.0, 279.0, "ledger")
    (PAPER_JUNIOR_LEGAL:      127.0, 203.0, "junior-legal" | "index-card")
    (PAPER_HALF_LETTER:       140.0, 216.0, "half-letter")
    (PAPER_GOVERNMENT_LETTER: 203.0, 267.0, "government-letter")
    (PAPER_GOVERNMENT_LEGAL:  216.0, 330.0, "government-legal" | "officio")

    // ANSI Extensions
    (PAPER_ANSI_C:        432.0, 559.0,  "ansi-c")
    (PAPER_ANSI_D:        559.0, 864.0,  "ansi-d")
    (PAPER_ANSI_E:        864.0, 1118.0, "ansi-e")
    (PAPER_ENGINEERING_F: 711.0, 1016.0, "engineering-f" | "engineering" | "navfac" | "aerospace")

    // Architectural Paper
    (PAPER_ARCH_A:  229.0, 305.0,  "arch-a" | "arch-1")
    (PAPER_ARCH_B:  305.0, 457.0,  "arch-b" | "arch-2" | "extra-tabloide")
    (PAPER_ARCH_C:  457.0, 610.0,  "arch-c" | "arch-3")
    (PAPER_ARCH_D:  610.0, 914.0,  "arch-d" | "arch-4")
    (PAPER_ARCH_E1: 762.0, 1067.0, "arch-e1" | "arch-5")
    (PAPER_ARCH_E:  914.0, 1219.0, "arch-e" | "arch-6")

    // ---------------------------------------------------------------------- //
    // Japan

    // JIS B Series
    (PAPER_JIS_B0:  1030.0, 1456.0, "jis-b0" | "jb0")
    (PAPER_JIS_B1:  728.0, 1030.0,  "jis-b1" | "jb1")
    (PAPER_JIS_B2:  515.0, 728.0,   "jis-b2" | "jb2")
    (PAPER_JIS_B3:  364.0, 515.0,   "jis-b3" | "jb3")
    (PAPER_JIS_B4:  257.0, 364.0,   "jis-b4" | "jb4")
    (PAPER_JIS_B5:  182.0, 257.0,   "jis-b5" | "jb5")
    (PAPER_JIS_B6:  128.0, 182.0,   "jis-b6" | "jb6")
    (PAPER_JIS_B7:  91.0,  128.0,   "jis-b7" | "jb7")
    (PAPER_JIS_B8:  64.0,  91.0,    "jis-b8" | "jb8")
    (PAPER_JIS_B9:  45.0,  64.0,    "jis-b9" | "jb9")
    (PAPER_JIS_B10: 32.0,  45.0,    "jis-b10" | "jb10")
    (PAPER_JIS_B11: 22.0,  32.0,    "jis-b11" | "jb11")

    // Traditional
    (PAPER_SHIROKU_BAN_4: 264.0, 379.0, "shiroku-ban-4")
    (PAPER_SHIROKU_BAN_5: 189.0, 262.0, "shiroku-ban-5")
    (PAPER_SHIROKU_BAN_6: 127.0, 188.0, "shiroku-ban-6")
    (PAPER_KIKU_4:        227.0, 306.0, "kiku-4")
    (PAPER_KIKU_5:        151.0, 227.0, "kiku-5")

    // ---------------------------------------------------------------------- //
    // China

    // Chinese D Series
    (PAPER_SAC_D0: 764.0, 1064.0, "sac-d0" | "cn-d0")
    (PAPER_SAC_D1: 532.0, 760.0,  "sac-d1" | "cn-d1")
    (PAPER_SAC_D2: 380.0, 528.0,  "sac-d2" | "cn-d2")
    (PAPER_SAC_D3: 264.0, 376.0,  "sac-d3" | "cn-d3")
    (PAPER_SAC_D4: 188.0, 260.0,  "sac-d4" | "cn-d4")
    (PAPER_SAC_D5: 130.0, 184.0,  "sac-d5" | "cn-d5")
    (PAPER_SAC_D6: 92.0, 126.0,   "sac-d6" | "cn-d6")

    // ---------------------------------------------------------------------- //
    // United Kingdom Imperial (Assortment)

    (PAPER_MONARCH:      184.0, 267.0,  "monarch")
    (PAPER_QUARTO:       229.0, 279.0,  "quarto" | "us-quarto")
    (PAPER_UK_QUARTO:    203.0, 254.0,  "uk-quarto" | "imperial-quarto")
    (PAPER_UK_FOOLSCAP:  343.0, 432.0,  "foolscap" | "us-foolscap")
    (PAPER_FOOLSCAP:     203.0, 330.0,  "imperial-foolscap" | "uk-foolscap")
    (PAPER_POTT:         318.0, 381.0,  "pott")
    (PAPER_CROWN:        318.0, 508.0,  "crown")
    (PAPER_PINCHED_POST: 375.0, 470.0,  "pinched-post")
    (PAPER_POST:         394.0, 489.0,  "post")
    (PAPER_LARGE_POST:   419.0, 533.0,  "large-post")
    (PAPER_DEMY:         445.0, 572.0,  "demy")
    (PAPER_ROYAL:        508.0, 635.0,  "royal")
    (PAPER_DOUBLE_CROWN: 508.0, 762.0,  "double-crown" | "theatre")
    (PAPER_ELEPHANT:     584.0, 711.0,  "elephant")
    (PAPER_DOUBLE_ROYAL: 635.0, 1016.0, "double-royal" | "rail")
    (PAPER_QUAD_CROWN:   762.0, 1016.0, "quad-crown" | "cinema")

    // ---------------------------------------------------------------------- //
    // French Traditional (AFNOR)

    (PAPER_CLOCHE:               300.0, 400.0,   "cloche")
    (PAPER_POT:                  310.0, 400.0,   "pot" | "ecolier" | "écolier")
    (PAPER_TELLIERE:             340.0, 440.0,   "telliere" | "tellière")
    (PAPER_COURONNE_ECRITURE:    360.0, 460.0,   "couronne-ecriture" | "couronne" | "couronne-écriture")
    (PAPER_COURONNE_EDITION:     370.0, 470.0,   "couronne-edition" | "couronne-édition")
    (PAPER_ROBERTO:              390.0, 500.0,   "roberto")
    (PAPER_ECU:                  400.0, 520.0,   "ecu" | "écu")
    (PAPER_COQUILLE:             440.0, 560.0,   "coquille")
    (PAPER_CARRE:                450.0, 560.0,   "carre" | "carré")
    (PAPER_CAVALIER:             460.0, 620.0,   "cavalier")
    (PAPER_DEMI_RAISIN:          325.0, 500.0,   "demi-raisin")
    (PAPER_RAISIN:               500.0, 650.0,   "raisin" | "dessin")
    (PAPER_DOUBLE_RAISIN:        650.0, 1000.0,  "double-raisin")
    (PAPER_JESUS:                560.0, 760.0,   "jesus" | "jésus")
    (PAPER_SOLEIL:               600.0, 800.0,   "soleil")
    (PAPER_COLOMBIER_AFFICHE:    600.0, 800.0,   "colombier-affiche" | "affiche")
    (PAPER_COLOMBIER_COMMERCIAL: 630.0, 900.0,   "colombier-commercial")
    (PAPER_PETIT_AIGLE:          700.0, 940.0,   "petit-aigle")
    (PAPER_GRAND_AIGLE:          750.0, 1060.0,  "grand-aigle" | "napoleon")
    (PAPER_GRAND_MONDE:          900.0, 1260.0,  "grand-monde")
    (PAPER_UNIVERS:              1000.0, 1300.0, "univers" | "universe")

    // ---------------------------------------------------------------------- //
    // Newspaper

    (PAPER_COMPACT:        280.0, 430.0, "compact")
    (PAPER_BERLINER:       315.0, 470.0, "berliner" | "midi")
    (PAPER_RHENISH:        350.0, 520.0, "rhenish")
    (PAPER_BROADSHEET:     381.0, 578.0, "broadsheet" | "newspaper")
    (PAPER_NEW_YORK_TIMES: 305.0, 559.0, "new-york-times" | "times")

    // ---------------------------------------------------------------------- //
    // Books

    (PAPER_FOLIO_BOOK:  304.8, 482.6,  "book-folio")
    (PAPER_QUARTO_BOOK: 241.3, 304.8,  "book-quarto")
    (PAPER_OCTAVO_BOOK: 152.4, 228.6,  "book-octavo")
    (PAPER_16_MO_BOOK:  101.6, 171.45, "book-16mo")
    (PAPER_32_MO_BOOK:  88.9, 139.7,   "book-32mo")

    // ---------------------------------------------------------------------- //
    // Various

    (PAPER_ID_1:             85.6, 53.98,  "id-card" | "id-1" | "iso-7810-id-1" | "eu-business-card" | "business-card")
    (PAPER_US_BUSINESS_CARD: 88.9, 50.8,   "us-business-card")
    (PAPER_JP_BUSINESS_CARD: 91.0, 55.0,   "jp-business-card")
    (PAPER_CN_BUSINESS_CARD: 90.0, 54.0,   "cn-business-card")
    (PAPER_A4_16_9:          297.0, 148.5, "presentation-4-3")
    (PAPER_A4_4_3:           280.0, 210.0, "presentation-16-9" | "presentation")
    (PAPER_POSTCARD:         101.6, 152.4, "postcard")
}
