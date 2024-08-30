//! Translations.

use crate::text::{Lang, Region};
use once_cell::sync::Lazy;

macro_rules! key {
    ($name:ident) => {
        fn $name(&self) -> Option<&'static str> {
            None
        }
    };

    ($name:ident = $default:ident) => {
        fn $name(&self) -> Option<&'static str> {
            self.$default()
        }
    };

    ($name:ident = $default:literal) => {
        fn $name(&self) -> Option<&'static str> {
            Some($default)
        }
    };
}

macro_rules! define_keys {
    {
        $vis:vis $Trait:ident {
            $( $(#[$attr:meta])* $key:ident $(= $value:tt)?; )*
        }
    } => {
        $vis trait $Trait {
            $( $(#[$attr:meta])* key!($key $(= $value)?); )*
        }

        impl<T: $Trait + ?Sized> $Trait for &T {
            $(
                fn $key(&self) -> Option<&'static str> {
                    T::$key(self)
                }
            )*
        }

        impl<T: $Trait> $Trait for Option<T> {
            $(
                fn $key(&self) -> Option<&'static str> {
                    match self {
                        None => None,
                        Some(x) => x.$key(),
                    }
                }
            )*
        }

        impl<T: $Trait, U: $Trait> $Trait for (T, U) {
            $(
                fn $key(&self) -> Option<&'static str> {
                    self.0.$key().or_else(|| self.1.$key())
                }
            )*
        }
    };
}

define_keys! {
    pub Translation {
        figure;
        figure_caption_separator;
        table;
        equation;
        bibliography;
        heading;
        outline;
        raw;
        left_single_quote = "‘";
        right_single_quote = "’";
        left_double_quote = "“";
        right_double_quote = "”";
        alternate_left_single_quote = left_single_quote;
        alternate_right_single_quote = right_single_quote;
        alternate_left_double_quote = left_double_quote;
        alternate_right_double_quote = right_double_quote;
    }
}

#[macro_export]
macro_rules! translation {
    { $(#[$attr:meta])* $vis:vis $name:ident { $( $key:ident = $value:expr; )* } } => {
        $( #[$attr] )*
        #[allow(non_camel_case_types)]
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        $vis struct $name;

        impl Translation for $name {
            $(
                fn $key(&self) -> Option<&'static str> {
                    Some($value.into())
                }
            )*
        }
    };
}

translation! {
    pub AR {
        figure = "شكل";
        table = "جدول";
        equation = "معادلة";
        bibliography = "المراجع";
        heading = "الفصل";
        outline = "المحتويات";
        raw = "قائمة";
    }
}

translation! {
    pub BS {
        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "”";
        right_double_quote = "”";
    }
}

translation! {
    pub CA {
        figure = "Figura";
        table = "Taula";
        equation = "Equació";
        bibliography = "Bibliografia";
        heading = "Secció";
        outline = "Índex";
        raw = "Llistat";
    }
}

translation! {
    pub CS {
        figure = "Obrázek";
        table = "Tabulka";
        equation = "Rovnice";
        bibliography = "Bibliografie";
        heading = "Kapitola";
        outline = "Obsah";
        raw = "Seznam";

        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
        alternate_left_single_quote = "›";
        alternate_right_single_quote = "‹";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "«";

    }
}

translation! {
    pub DA {
        figure = "Figur";
        table = "Tabel";
        equation = "Ligning";
        bibliography = "Bibliografi";
        heading = "Afsnit";
        outline = "Indhold";
        raw = "Liste";

        left_single_quote = "‘";
        right_single_quote = "’";
        left_double_quote = "“";
        right_double_quote = "”";
        alternate_left_single_quote = "›";
        alternate_right_single_quote = "‹";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "«";

    }
}

translation! {
    pub DE {
        figure = "Abbildung";
        table = "Tabelle";
        equation = "Gleichung";
        bibliography = "Bibliographie";
        heading = "Abschnitt";
        outline = "Inhaltsverzeichnis";
        raw = "Listing";

        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
        alternate_left_single_quote = "›";
        alternate_right_single_quote = "‹";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "«";

    }
}

translation! {
    pub DE_CH {
        left_single_quote = "‹";
        right_single_quote = "›";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‚";
        alternate_right_single_quote = "‘";
        alternate_left_double_quote = "„";
        alternate_right_double_quote = "“";
    }
}

translation! {
    pub DE_LI {
        left_single_quote = "‹";
        right_single_quote = "›";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‚";
        alternate_right_single_quote = "‘";
        alternate_left_double_quote = "„";
        alternate_right_double_quote = "“";
    }
}

translation! {
    /// English translation.
    ///
    /// Should never return [`None`].
    pub EN {
        figure = "Figure";
        table = "Table";
        equation = "Equation";
        bibliography = "Bibliography";
        heading = "Section";
        outline = "Contents";
        raw = "Listing";

        figure_caption_separator = ": ";

        left_single_quote = "‘";
        right_single_quote = "’";
        left_double_quote = "“";
        right_double_quote = "”";
    }
}

translation! {
    pub ES {
        figure = "Figura";
        table = "Tabla";
        equation = "Ecuación";
        bibliography = "Bibliografía";
        heading = "Sección";
        outline = "Índice";
        raw = "Listado";
    }
}

translation! {
    pub ES_ES {
        left_single_quote = "“";
        right_single_quote = "”";
        left_double_quote = "«";
        right_double_quote = "»";
    }
}

translation! {
    pub ET {
        figure = "Joonis";
        table = "Tabel";
        equation = "Valem";
        bibliography = "Viited";
        heading = "Peatükk";
        outline = "Sisukord";
        raw = "List";

        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
    }
}

translation! {
    pub FI {
        figure = "Kuva";
        table = "Taulukko";
        equation = "Yhtälö";
        bibliography = "Viitteet";
        heading = "Osio";
        outline = "Sisällys";
        raw = "Esimerkki";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "”";
        right_double_quote = "”";
        alternate_left_single_quote = "’";
        alternate_right_single_quote = "’";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "»";
    }
}

translation! {
    pub FR {
        figure = "Fig.";
        table = "Tableau";
        equation = "Équation";
        bibliography = "Bibliographie";
        heading = "Chapitre";
        outline = "Table des matières";
        raw = "Liste";

        figure_caption_separator = ".\u{a0}– ";

        left_single_quote = "‹\u{a0}";
        right_single_quote = "\u{a0}›";
        left_double_quote = "«\u{a0}";
        right_double_quote = "\u{a0}»";
        alternate_left_single_quote = "‘";
        alternate_right_single_quote = "’";
        alternate_left_double_quote = "“";
        alternate_right_double_quote = "”";
    }
}

translation! {
    pub GL {
        figure = "Figura";
        table = "Táboa";
        equation = "Ecuación";
        bibliography = "Bibliografía";
        heading = "Sección";
        outline = "Índice";
        raw = "Listado";
    }
}

translation! {
    pub GR {
        figure = "Σχήμα";
        table = "Πίνακας";
        equation = "Εξίσωση";
        bibliography = "Βιβλιογραφία";
        heading = "Κεφάλαιο";
        outline = "Περιεχόμενα";
        raw = "Παράθεση";

        left_single_quote = "‘";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
    }
}

translation! {
    pub HU {
        figure = "Ábra";
        table = "Táblázat";
        equation = "Egyenlet";
        bibliography = "Irodalomjegyzék";
        heading = "Fejezet";
        outline = "Tartalomjegyzék";
        // raw = "";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "„";
        right_double_quote = "”";
    }
}

translation! {
    pub IS {
        figure = "Mynd";
        table = "Tafla";
        equation = "Jafna";
        bibliography = "Heimildaskrá";
        heading = "Kafli";
        outline = "Efnisyfirlit";
        raw = "Sýnishorn";

        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
    }
}

translation! {
    pub IT {
        figure = "Figura";
        table = "Tabella";
        equation = "Equazione";
        bibliography = "Bibliografia";
        heading = "Sezione";
        outline = "Indice";
        raw = "Codice";

        left_single_quote = "“";
        right_single_quote = "”";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‘";
        alternate_right_single_quote = "’";
        alternate_left_double_quote = "“";
        alternate_right_double_quote = "”";
    }
}

translation! {
    pub JA {
        figure = "図";
        table = "表";
        equation = "式";
        bibliography = "参考文献";
        heading = "節";
        outline = "目次";
        raw = "リスト";
    }
}

translation! {
    pub LA {
        figure = "Descriptio";
        table = "Tabula";
        equation = "Equatio";
        bibliography = "Conspectus librorum";
        heading = "Caput";
        outline = "Index capitum";
        raw = "Exemplum";

        left_single_quote = "“";
        right_single_quote = "”";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_double_quote = "«\u{202f}";
        alternate_right_double_quote = "\u{202f}»";
    }
}

translation! {
    pub LT {
        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
    }
}

translation! {
    pub LV {
        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
    }
}

translation! {
    pub NB {
        figure = "Figur";
        table = "Tabell";
        equation = "Ligning";
        bibliography = "Bibliografi";
        heading = "Kapittel";
        outline = "Innhold";
        raw = "Utskrift";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‚";
        alternate_right_single_quote = "‘";
        alternate_left_double_quote = "„";
        alternate_right_double_quote = "“";
    }
}

translation! {
    pub NL {
        figure = "Figuur";
        table = "Tabel";
        equation = "Vergelijking";
        bibliography = "Bibliografie";
        heading = "Hoofdstuk";
        outline = "Inhoudsopgave";
        raw = "Listing";
    }
}

translation! {
    pub NN {
        figure = "Figur";
        table = "Tabell";
        equation = "Likning";
        bibliography = "Bibliografi";
        heading = "Kapittel";
        outline = "Innhald";
        raw = "Utskrift";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‚";
        alternate_right_single_quote = "‘";
        alternate_left_double_quote = "„";
        alternate_right_double_quote = "“";
    }
}

translation! {
    pub NO {
        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‚";
        alternate_right_single_quote = "‘";
        alternate_left_double_quote = "„";
        alternate_right_double_quote = "“";
    }
}

translation! {
    pub PL {
        figure = "Rysunek";
        table = "Tabela";
        equation = "Równanie";
        bibliography = "Bibliografia";
        heading = "Sekcja";
        outline = "Spis treści";
        raw = "Program";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "„";
        right_double_quote = "”";
    }
}

translation! {
    pub PT {
        figure = "Figura";
        table = "Tabela";
        equation = "Equação";
        bibliography = "Bibliografia";
        heading = "Seção";
        outline = "Sumário";
        raw = "Listagem";
    }
}

translation! {
    pub PT_PT {
        heading = "Secção";
        outline = "Índice";
    }
}

translation! {
    pub RO {
        figure = "Figura";
        table = "Tabelul";
        equation = "Ecuația";
        bibliography = "Bibliografie";
        heading = "Secțiunea";
        outline = "Cuprins";
        // May be wrong.
        raw = "Listă";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "„";
        right_double_quote = "”";
    }
}

translation! {
    pub RU {
        figure = "Рис.";
        table = "Таблица";
        equation = "Уравнение";
        bibliography = "Библиография";
        heading = "Раздел";
        outline = "Содержание";
        raw = "Листинг";

        figure_caption_separator = ". ";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
        alternate_left_single_quote = "‘";
        alternate_right_single_quote = "’";
        alternate_left_double_quote = "“";
        alternate_right_double_quote = "”";
    }
}

translation! {
    pub SK {
        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
        alternate_left_single_quote = "›";
        alternate_right_single_quote = "‹";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "«";
    }
}

translation! {
    pub SL {
        figure = "Slika";
        table = "Tabela";
        equation = "Enačba";
        bibliography = "Literatura";
        heading = "Poglavje";
        outline = "Kazalo";
        raw = "Program";

        left_single_quote = "‚";
        right_single_quote = "‘";
        left_double_quote = "„";
        right_double_quote = "“";
        alternate_left_single_quote = "›";
        alternate_right_single_quote = "‹";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "«";
    }
}

translation! {
    pub SQ {
        figure = "Figurë";
        table = "Tabel";
        equation = "Ekuacion";
        bibliography = "Bibliografi";
        heading = "Kapitull";
        outline = "Përmbajtja";
        raw = "List";
    }
}

translation! {
    pub SR {
        figure = "Слика";
        table = "Табела";
        equation = "Једначина";
        bibliography = "Литература";
        heading = "Поглавље";
        outline = "Садржај";
        raw = "Програм";
    }
}

translation! {
    pub SV {
        figure = "Figur";
        table = "Tabell";
        equation = "Ekvation";
        bibliography = "Bibliografi";
        heading = "Kapitel";
        outline = "Innehåll";
        raw = "Listing";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "”";
        right_double_quote = "”";
        alternate_left_single_quote = "’";
        alternate_right_single_quote = "’";
        alternate_left_double_quote = "»";
        alternate_right_double_quote = "»";
    }
}

translation! {
    pub TL {
        figure = "Pigura";
        table = "Talaan";
        equation = "Ekwasyon";
        bibliography = "Bibliograpiya";
        heading = "Seksyon";
        outline = "Talaan ng mga Nilalaman";
        raw = "Listahan";
    }
}

translation! {
    pub TR {
        figure = "Şekil";
        table = "Tablo";
        equation = "Denklem";
        bibliography = "Kaynakça";
        heading = "Bölüm";
        outline = "İçindekiler";
        raw = "Liste";
    }
}

translation! {
    pub UK {
        figure = "Рисунок";
        table = "Таблиця";
        equation = "Рівняння";
        bibliography = "Бібліографія";
        heading = "Розділ";
        outline = "Зміст";
        raw = "Лістинг";

        left_single_quote = "’";
        right_single_quote = "’";
        left_double_quote = "«";
        right_double_quote = "»";
    }
}

translation! {
    pub VI {
        figure = "Hình";
        table = "Bảng";
        equation = "Phương trình";
        bibliography = "Tài liệu tham khảo";
        heading = "Phần";
        outline = "Mục lục";
        // May be wrong.
        raw = "Chương trình";
    }
}

translation! {
    pub ZH {
        figure = "图";
        table = "表";
        equation = "式";
        bibliography = "参考文献";
        heading = "小节";
        outline = "目录";
        raw = "代码";

        figure_caption_separator = "\u{2003}";
    }
}

translation! {
    pub ZH_TW {
        figure = "圖";
        // table = "";
        equation = "式";
        bibliography = "書目";
        heading = "小節";
        outline = "目錄";
        raw = "程式";
    }
}

/// A [natural language](Lang) and an optional [region](Region).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
struct Locale(Lang, Option<Region>);

impl Locale {
    /// Creates a new locale with the same language, but no region.
    pub const fn without_region(&self) -> Self {
        Self(self.0, None)
    }
}

macro_rules! locale {
    ($lang:ident-$region:ident, $translation:expr) => {
        (
            Locale(
                stringify!($lang).parse().unwrap(),
                Some(stringify!($region).parse().unwrap()),
            ),
            &$translation as &(dyn Translation + Sync),
        )
    };

    ($lang:ident, $translation:expr) => {
        (
            Locale(stringify!($lang).parse().unwrap(), None),
            &$translation as &(dyn Translation + Sync),
        )
    };
}

static TRANSLATIONS: Lazy<[(Locale, &(dyn Translation + Sync)); 43]> = Lazy::new(|| {
    let mut translations = [
        locale!(ar, AR),
        locale!(bs, BS),
        locale!(ca, CA),
        locale!(cs, CS),
        locale!(da, DA),
        locale!(de, DE),
        locale!(de - CH, DE_CH),
        locale!(de - LI, DE_LI),
        locale!(en, EN),
        locale!(es, ES),
        locale!(es - ES, ES_ES),
        locale!(et, ET),
        locale!(fi, FI),
        locale!(fr, FR),
        locale!(gl, GL),
        locale!(gr, GR),
        locale!(hu, HU),
        locale!(is, IS),
        locale!(it, IT),
        locale!(ja, JA),
        locale!(la, LA),
        locale!(lt, LT),
        locale!(lv, LV),
        locale!(nb, NB),
        locale!(nl, NL),
        locale!(nn, NN),
        locale!(no, NO),
        locale!(pl, PL),
        locale!(pt, PT),
        locale!(pt - PT, PT_PT),
        locale!(ro, RO),
        locale!(ru, RU),
        locale!(sk, SK),
        locale!(sl, SL),
        locale!(sq, SQ),
        locale!(sr, SR),
        locale!(sv, SV),
        locale!(tl, TL),
        locale!(tr, TR),
        locale!(uk, UK),
        locale!(vi, VI),
        locale!(zh, ZH),
        locale!(zh - TW, ZH_TW),
    ];
    translations.sort_by_key(|&(locale, _)| locale);
    translations
});

fn find_exact_translation(locale: Locale) -> Option<&'static (dyn Translation + Sync)> {
    TRANSLATIONS
        .binary_search_by_key(&locale, |&(loc, _)| loc)
        .ok()
        .map(|i| TRANSLATIONS[i].1)
}

/// Retrieves the translation for a specific locale (i.e., lang + optional
/// region).
///
/// Keys that are not translated will return [`None`].
fn translation_cascade(
    lang: Lang,
    region: Option<Region>,
) -> impl Translation + Copy + Sync {
    let locale = Locale(lang, region);
    (
        region.and_then(|_| find_exact_translation(locale)),
        find_exact_translation(locale.without_region()),
    )
}

/// The default translation. Returns [`Some`] for all keys.
pub const DEFAULT_TRANSLATION: EN = EN;

/// Retrieves the translation for a specific locale (i.e., lang + optional
/// region).
///
/// The returned translation falls back to [`DEFAULT_TRANSLATION`] for keys that
/// don't have translations in the desired locale. This means keys should never
/// return [`None`].
pub fn defaulted_translation_cascade(
    lang: Lang,
    region: Option<Region>,
) -> impl Translation + Copy + Sync {
    (translation_cascade(lang, region), DEFAULT_TRANSLATION)
}
