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

/// The size of a page with the given name.
pub fn parse_paper_name(paper: &str) -> ParseResult<Size2D> {
    Ok(match paper.to_lowercase().as_ref() {
        "a0" | "poster" => PAPER_A0,
        "a1" => PAPER_A1,
        "a2" => PAPER_A2,
        "a3" => PAPER_A3,
        "a4" => PAPER_A4,
        "a5" => PAPER_A5,
        "a6" => PAPER_A6,
        "a7" | "iso-7810-id-2" | "id-2" | "visa" | "flyer" => PAPER_A7,
        "a8" => PAPER_A8,
        "a9" => PAPER_A9,
        "a10" => PAPER_A10,
        "a11" => PAPER_A11,
        "b1" | "flipchart" => PAPER_B1,
        "b2" => PAPER_B2,
        "b3" => PAPER_B3,
        "b4" | "sheet-music" => PAPER_B4,
        "b5" | "book" => PAPER_B5,
        "b6" => PAPER_B6,
        "b7" | "passport" | "iso-7810-id-3" | "id-3" => PAPER_B7,
        "b8" => PAPER_B8,
        "c3" => PAPER_C3,
        "c4" => PAPER_C4,
        "c5" => PAPER_C5,
        "c6" => PAPER_C6,
        "c7" => PAPER_C7,
        "c8" => PAPER_C8,
        "din-d3" => PAPER_D3,
        "din-d4" => PAPER_D4,
        "din-d5" | "dvd" => PAPER_D5,
        "din-d6" => PAPER_D6,
        "din-d7" => PAPER_D7,
        "din-d8" => PAPER_D8,
        "sis-g5" => PAPER_G5,
        "sis-e5" => PAPER_E5,
        "folio" | "us-folio" | "us-f4" => PAPER_FOLIO,
        "legal" => PAPER_LEGAL,
        "ledger" => PAPER_LEDGER,
        "junior-legal" | "index-card" => PAPER_JUNIOR_LEGAL,
        "half-letter" => PAPER_HALF_LETTER,
        "government-letter" => PAPER_GOVERNMENT_LETTER,
        "government-legal" | "officio" => PAPER_GOVERNMENT_LEGAL,
        "letter" | "ansi-a" | "american-quarto" | "carta" => PAPER_LETTER,
        "tabloid" | "ansi-b" => PAPER_TABLOID,
        "ansi-c" => PAPER_ANSI_C,
        "ansi-d" => PAPER_ANSI_D,
        "ansi-e" => PAPER_ANSI_E,
        "engineering-f" | "engineering" | "navfac" | "aerospace" => PAPER_ENGINEERING_F,
        "arch-a" | "arch-1" => PAPER_ARCH_A,
        "arch-b" | "arch-2" | "extra-tabloide" => PAPER_ARCH_B,
        "arch-c" | "arch-3" => PAPER_ARCH_C,
        "arch-d" | "arch-4" => PAPER_ARCH_D,
        "arch-e1" | "arch-5" => PAPER_ARCH_E1,
        "arch-e" | "arch-6" => PAPER_ARCH_E,
        "jis-b0" | "jb0" => PAPER_JIS_B0,
        "jis-b1" | "jb1" => PAPER_JIS_B1,
        "jis-b2" | "jb2" => PAPER_JIS_B2,
        "jis-b3" | "jb3" => PAPER_JIS_B3,
        "jis-b4" | "jb4" => PAPER_JIS_B4,
        "jis-b5" | "jb5" => PAPER_JIS_B5,
        "jis-b6" | "jb6" => PAPER_JIS_B6,
        "jis-b7" | "jb7" => PAPER_JIS_B7,
        "jis-b8" | "jb8" => PAPER_JIS_B8,
        "jis-b9" | "jb9" => PAPER_JIS_B9,
        "jis-b10" | "jb10" => PAPER_JIS_B10,
        "jis-b11" | "jb11" => PAPER_JIS_B11,
        "shiroku-ban-4" => PAPER_SHIROKU_BAN_4,
        "shiroku-ban-5" => PAPER_SHIROKU_BAN_5,
        "shiroku-ban-6" => PAPER_SHIROKU_BAN_6,
        "kiku-4" => PAPER_KIKU_4,
        "kiku-5" => PAPER_KIKU_5,
        "sac-d0" | "cn-d0" => PAPER_SAC_D0,
        "sac-d1" | "cn-d1" => PAPER_SAC_D1,
        "sac-d2" | "cn-d2" => PAPER_SAC_D2,
        "sac-d3" | "cn-d3" => PAPER_SAC_D3,
        "sac-d4" | "cn-d4" => PAPER_SAC_D4,
        "sac-d5" | "cn-d5" => PAPER_SAC_D5,
        "sac-d6" | "cn-d6" => PAPER_SAC_D6,
        "monarch" => PAPER_MONARCH,
        "quarto" | "us-quarto" => PAPER_QUARTO,
        "uk-quarto" | "imperial-quarto" => PAPER_QUARTO,
        "foolscap" | "us-foolscap" => PAPER_FOOLSCAP,
        "imperial-foolscap" | "uk-foolscap" => PAPER_UK_FOOLSCAP,
        "pott" => PAPER_POTT,
        "crown" => PAPER_CROWN,
        "pinched-post" => PAPER_PINCHED_POST,
        "post" => PAPER_POST,
        "large-post" => PAPER_LARGE_POST,
        "demy" => PAPER_DEMY,
        "royal" => PAPER_ROYAL,
        "double-crown" | "theatre" => PAPER_DOUBLE_CROWN,
        "elephant" => PAPER_ELEPHANT,
        "double-royal" | "rail" => PAPER_DOUBLE_ROYAL,
        "quad-crown" | "cinema" => PAPER_QUAD_CROWN,
        "cloche" => PAPER_CLOCHE,
        "pot" | "ecolier" | "écolier" => PAPER_POT,
        "telliere" | "tellière" => PAPER_TELLIERE,
        "couronne-ecriture" | "couronne" | "couronne-écriture" => PAPER_COURONNE_ECRITURE,
        "couronne-edition" | "couronne-édition" => PAPER_COURONNE_EDITION,
        "roberto" => PAPER_ROBERTO,
        "ecu" | "écu" => PAPER_ECU,
        "coquille" => PAPER_COQUILLE,
        "carre" | "carré" => PAPER_CARRE,
        "cavalier" => PAPER_CAVALIER,
        "demi-raisin" => PAPER_DEMI_RAISIN,
        "raisin" | "dessin" => PAPER_RAISIN,
        "double-raisin" => PAPER_DOUBLE_RAISIN,
        "jesus" | "jésus" => PAPER_JESUS,
        "soleil" => PAPER_SOLEIL,
        "colombier-affiche" | "affiche" => PAPER_COLOMBIER_AFFICHE,
        "colombier-commercial" => PAPER_COLOMBIER_COMMERCIAL,
        "petit-aigle" => PAPER_PETIT_AIGLE,
        "grand-aigle" | "napoleon" => PAPER_GRAND_AIGLE,
        "grand-monde" => PAPER_GRAND_MONDE,
        "univers" | "universe" => PAPER_UNIVERS,
        "compact" => PAPER_COMPACT,
        "berliner" | "midi" => PAPER_BERLINER,
        "rhenish" => PAPER_RHENISH,
        "broadsheet" | "newspaper" => PAPER_BROADSHEET,
        "new-york-times" | "times" => PAPER_NEW_YORK_TIMES,
        "book-folio" => PAPER_FOLIO_BOOK,
        "book-quarto" => PAPER_QUARTO_BOOK,
        "book-octavo" => PAPER_OCTAVO_BOOK,
        "book-16mo" => PAPER_16_MO_BOOK,
        "book-32mo" => PAPER_32_MO_BOOK,
        "id-card" | "id-1" | "iso-7810-id-1" | "eu-business-card" | "business-card" => PAPER_ID_1,
        "us-business-card" => PAPER_US_BUSINESS_CARD,
        "jp-business-card" => PAPER_JP_BUSINESS_CARD,
        "cn-business-card" => PAPER_CN_BUSINESS_CARD,
        "presentation-4-3" => PAPER_A4_4_3,
        "presentation-16-9" | "presentation" => PAPER_A4_16_9,
        "postcard" => PAPER_POSTCARD,

        _ => error!("unknown paper size: `{}`", paper),
    })
}

macro_rules! paper {
    ($var:ident: $width:expr, $height: expr) => {
        /// The size of the paper that's in the name.
        pub const $var: Size2D = Size2D {
            x: Size { points: 2.83465 * $width },
            y: Size { points: 2.83465 * $height },
        };
    };
}

// ** Paper sizes in mm.
// * ISO 216
paper!(PAPER_A0:  841.0, 1189.0);
paper!(PAPER_A1:  594.0, 841.0);
paper!(PAPER_A2:  420.0, 594.0);
paper!(PAPER_A3:  297.0, 420.0);
paper!(PAPER_A4:  210.0, 297.0);
paper!(PAPER_A5:  148.0, 210.0);
paper!(PAPER_A6:  105.0, 148.0);
paper!(PAPER_A7:  74.0,  105.0);
paper!(PAPER_A8:  52.0,  74.0);
paper!(PAPER_A9:  37.0,  52.0);
paper!(PAPER_A10: 26.0,  37.0);
paper!(PAPER_A11: 18.0,  26.0);
// * B Series
paper!(PAPER_B1:  707.0, 1000.0);
paper!(PAPER_B2:  500.0, 707.0);
paper!(PAPER_B3:  353.0, 500.0);
paper!(PAPER_B4:  250.0, 353.0);
paper!(PAPER_B5:  176.0, 250.0);
paper!(PAPER_B6:  125.0, 176.0);
paper!(PAPER_B7:  88.0,  125.0);
paper!(PAPER_B8:  62.0,  88.0);
// * C Series
paper!(PAPER_C3:  324.0, 458.0);
paper!(PAPER_C4:  229.0, 324.0);
paper!(PAPER_C5:  162.0, 229.0);
paper!(PAPER_C6:  114.0, 162.0);
paper!(PAPER_C7:  81.0, 114.0);
paper!(PAPER_C8:  57.0, 81.0);
// * D Series (DIN extension to ISO)
paper!(PAPER_D3:  272.0, 385.0);
paper!(PAPER_D4:  192.0, 272.0);
paper!(PAPER_D5:  136.0, 192.0);
paper!(PAPER_D6:  96.0, 136.0);
paper!(PAPER_D7:  68.0,  96.0);
paper!(PAPER_D8:  48.0,  68.0);
// * Academically relevant SIS extensions
paper!(PAPER_G5:  169.0, 239.0);
paper!(PAPER_E5:  115.0, 220.0);

// ** US
// * Customary
paper!(PAPER_FOLIO:             210.0, 330.0);
paper!(PAPER_LETTER:            216.0, 279.0);
paper!(PAPER_LEGAL:             216.0, 356.0);
paper!(PAPER_TABLOID:           279.0, 432.0);
paper!(PAPER_LEDGER:            432.0, 279.0);
paper!(PAPER_JUNIOR_LEGAL:      127.0, 203.0);
paper!(PAPER_HALF_LETTER:       140.0, 216.0);
paper!(PAPER_GOVERNMENT_LETTER: 203.0, 267.0);
paper!(PAPER_GOVERNMENT_LEGAL:  216.0, 330.0);
// * ANSI Extensions
paper!(PAPER_ANSI_C:            432.0, 559.0);
paper!(PAPER_ANSI_D:            559.0, 864.0);
paper!(PAPER_ANSI_E:            864.0, 1118.0);
paper!(PAPER_ENGINEERING_F:     711.0, 1016.0);
// * Architectural Paper
paper!(PAPER_ARCH_A:   229.0, 305.0);
paper!(PAPER_ARCH_B:   305.0, 457.0);
paper!(PAPER_ARCH_C:   457.0, 610.0);
paper!(PAPER_ARCH_D:   610.0, 914.0);
paper!(PAPER_ARCH_E1:  762.0, 1067.0);
paper!(PAPER_ARCH_E:   914.0, 1219.0);

// ** Japanese
// * JIS B Series
paper!(PAPER_JIS_B0:  1030.0, 1456.0);
paper!(PAPER_JIS_B1:  728.0, 1030.0);
paper!(PAPER_JIS_B2:  515.0, 728.0);
paper!(PAPER_JIS_B3:  364.0, 515.0);
paper!(PAPER_JIS_B4:  257.0, 364.0);
paper!(PAPER_JIS_B5:  182.0, 257.0);
paper!(PAPER_JIS_B6:  128.0, 182.0);
paper!(PAPER_JIS_B7:  91.0,  128.0);
paper!(PAPER_JIS_B8:  64.0,  91.0);
paper!(PAPER_JIS_B9:  45.0,  64.0);
paper!(PAPER_JIS_B10:  32.0,  45.0);
paper!(PAPER_JIS_B11:  22.0,  32.0);
// * Traditional
paper!(PAPER_SHIROKU_BAN_4:  264.0, 379.0);
paper!(PAPER_SHIROKU_BAN_5:  189.0, 262.0);
paper!(PAPER_SHIROKU_BAN_6:  127.0, 188.0);
paper!(PAPER_KIKU_4:  227.0, 306.0);
paper!(PAPER_KIKU_5:  151.0, 227.0);

// ** Chinese D Series
paper!(PAPER_SAC_D0:  764.0, 1064.0);
paper!(PAPER_SAC_D1:  532.0, 760.0);
paper!(PAPER_SAC_D2:  380.0, 528.0);
paper!(PAPER_SAC_D3:  264.0, 376.0);
paper!(PAPER_SAC_D4:  188.0, 260.0);
paper!(PAPER_SAC_D5:  130.0, 184.0);
paper!(PAPER_SAC_D6:  92.0, 126.0);

// ** UK Imperial (assortment)
paper!(PAPER_MONARCH:      184.0, 267.0);
paper!(PAPER_QUARTO:       229.0, 279.0);
paper!(PAPER_UK_QUARTO:    203.0, 254.0);
paper!(PAPER_UK_FOOLSCAP:  343.0, 432.0);
paper!(PAPER_FOOLSCAP:     203.0, 330.0);
paper!(PAPER_POTT:         318.0, 381.0);
paper!(PAPER_CROWN:        318.0, 508.0);
paper!(PAPER_PINCHED_POST: 375.0, 470.0);
paper!(PAPER_POST:         394.0, 489.0);
paper!(PAPER_LARGE_POST:   419.0, 533.0);
paper!(PAPER_DEMY:         445.0, 572.0);
paper!(PAPER_ROYAL:        508.0, 635.0);
paper!(PAPER_DOUBLE_CROWN: 508.0, 762.0);
paper!(PAPER_ELEPHANT:     584.0, 711.0);
paper!(PAPER_DOUBLE_ROYAL: 635.0, 1016.0);
paper!(PAPER_QUAD_CROWN:   762.0, 1016.0);

// ** French Traditional (AFNOR)
paper!(PAPER_CLOCHE:               300.0, 400.0);
paper!(PAPER_POT:                  310.0, 400.0);
paper!(PAPER_TELLIERE:             340.0, 440.0);
paper!(PAPER_COURONNE_ECRITURE:    360.0, 460.0);
paper!(PAPER_COURONNE_EDITION:     370.0, 470.0);
paper!(PAPER_ROBERTO:              390.0, 500.0);
paper!(PAPER_ECU:                  400.0, 520.0);
paper!(PAPER_COQUILLE:             440.0, 560.0);
paper!(PAPER_CARRE:                450.0, 560.0);
paper!(PAPER_CAVALIER:             460.0, 620.0);
paper!(PAPER_DEMI_RAISIN:          325.0, 500.0);
paper!(PAPER_RAISIN:               500.0, 650.0);
paper!(PAPER_DOUBLE_RAISIN:        650.0, 1000.0);
paper!(PAPER_JESUS:                560.0, 760.0);
paper!(PAPER_SOLEIL:               600.0, 800.0);
paper!(PAPER_COLOMBIER_AFFICHE:    600.0, 800.0);
paper!(PAPER_COLOMBIER_COMMERCIAL: 630.0, 900.0);
paper!(PAPER_PETIT_AIGLE:          700.0, 940.0);
paper!(PAPER_GRAND_AIGLE:          750.0, 1060.0);
paper!(PAPER_GRAND_MONDE:          900.0, 1260.0);
paper!(PAPER_UNIVERS:              1000.0, 1300.0);

// ** Newspaper
paper!(PAPER_COMPACT:        280.0, 430.0);
paper!(PAPER_BERLINER:       315.0, 470.0);
paper!(PAPER_RHENISH:        350.0, 520.0);
paper!(PAPER_BROADSHEET:     381.0, 578.0);
paper!(PAPER_NEW_YORK_TIMES: 305.0, 559.0);

// ** Books
paper!(PAPER_FOLIO_BOOK:  304.8, 482.6);
paper!(PAPER_QUARTO_BOOK: 241.3, 304.8);
paper!(PAPER_OCTAVO_BOOK: 152.4, 228.6);
paper!(PAPER_16_MO_BOOK:  101.6, 171.45);
paper!(PAPER_32_MO_BOOK:  88.9, 139.7);

// ** Various
paper!(PAPER_ID_1: 85.6, 53.98);
paper!(PAPER_US_BUSINESS_CARD: 88.9, 50.8);
paper!(PAPER_JP_BUSINESS_CARD: 91.0, 55.0);
paper!(PAPER_CN_BUSINESS_CARD: 90.0, 54.0);
paper!(PAPER_A4_16_9: 297.0, 148.5);
paper!(PAPER_A4_4_3: 280.0, 210.0);
paper!(PAPER_POSTCARD:  101.6, 152.4);

