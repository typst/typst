use std::str::FromStr;

use ecow::{EcoString, eco_format};
use rustc_hash::FxHashMap;

use crate::diag::Hint;
use crate::foundations::{StyleChain, cast};
use crate::layout::Dir;
use crate::text::TextElem;

macro_rules! translation {
    ($lang:literal) => {
        ($lang, include_str!(concat!("../../translations/", $lang, ".txt")))
    };
}

const TRANSLATIONS: &[(&str, &str)] = &[
    translation!("af"),
    translation!("alt"),
    translation!("am"),
    translation!("ar"),
    translation!("as"),
    translation!("ast"),
    translation!("az"),
    translation!("be"),
    translation!("bg"),
    translation!("bn"),
    translation!("bo"),
    translation!("br"),
    translation!("bs"),
    translation!("bua"),
    translation!("ca"),
    translation!("ckb"),
    translation!("cs"),
    translation!("cu"),
    translation!("cy"),
    translation!("da"),
    translation!("de"),
    translation!("dsb"),
    translation!("el"),
    translation!("en"),
    translation!("eo"),
    translation!("es"),
    translation!("et"),
    translation!("eu"),
    translation!("fa"),
    translation!("fi"),
    translation!("fil"),
    translation!("fr"),
    translation!("fr-CA"),
    translation!("fur"),
    translation!("ga"),
    translation!("gd"),
    translation!("gl"),
    translation!("grc"),
    translation!("gu"),
    translation!("ha"),
    translation!("he"),
    translation!("hi"),
    translation!("hr"),
    translation!("hsb"),
    translation!("hu"),
    translation!("hy"),
    translation!("ia"),
    translation!("id"),
    translation!("is"),
    translation!("isv"),
    translation!("it"),
    translation!("ja"),
    translation!("ka"),
    translation!("km"),
    translation!("kmr"),
    translation!("kn"),
    translation!("ko"),
    translation!("ku"),
    translation!("la"),
    translation!("lb"),
    translation!("lo"),
    translation!("lt"),
    translation!("lv"),
    translation!("mk"),
    translation!("ml"),
    translation!("mr"),
    translation!("ms"),
    translation!("nb"),
    translation!("nl"),
    translation!("nn"),
    translation!("no"),
    translation!("oc"),
    translation!("or"),
    translation!("pa"),
    translation!("pl"),
    translation!("pms"),
    translation!("pt"),
    translation!("pt-PT"),
    translation!("rm"),
    translation!("ro"),
    translation!("ru"),
    translation!("se"),
    translation!("si"),
    translation!("sk"),
    translation!("sl"),
    translation!("sq"),
    translation!("sr"),
    translation!("sv"),
    translation!("ta"),
    translation!("te"),
    translation!("th"),
    translation!("tk"),
    translation!("tl"),
    translation!("tr"),
    translation!("ug"),
    translation!("uk"),
    translation!("ur"),
    translation!("vi"),
    translation!("zh"),
    translation!("zh-TW"),
];

/// A locale consisting of a language and an optional region.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Locale {
    pub lang: Lang,
    pub region: Option<Region>,
}

impl Default for Locale {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl Locale {
    pub const DEFAULT: Self = Self::new(Lang::ENGLISH, None);

    pub const fn new(lang: Lang, region: Option<Region>) -> Self {
        Locale { lang, region }
    }

    pub fn get_in(styles: StyleChain) -> Self {
        Locale::new(styles.get(TextElem::lang), styles.get(TextElem::region))
    }

    pub fn rfc_3066(self) -> EcoString {
        let mut buf = EcoString::from(self.lang.as_str());
        if let Some(region) = self.region {
            buf.push('-');
            buf.push_str(region.as_str());
        }
        buf
    }
}

/// An identifier for a natural language.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Lang([u8; 3], u8);

impl Lang {
    pub const ABKHAZIAN: Self = Self(*b"ab ", 2);
    pub const AFAR: Self = Self(*b"aa ", 2);
    pub const AFRIKAANS: Self = Self(*b"af ", 2);
    pub const AGHEM: Self = Self(*b"agq", 3);
    pub const AKAN: Self = Self(*b"ak ", 2);
    pub const AKKADIAN: Self = Self(*b"akk", 3);
    pub const ALBANIAN: Self = Self(*b"sq ", 2);
    pub const ALGERIAN_ARABIC: Self = Self(*b"arq", 3);
    pub const AMHARIC: Self = Self(*b"am ", 2);
    pub const ANCIENT_EGYPTIAN: Self = Self(*b"egy", 3);
    pub const ANCIENT_GREEK: Self = Self(*b"grc", 3);
    pub const ANCIENT_HEBREW: Self = Self(*b"hbo", 3);
    pub const ARABIC: Self = Self(*b"ar ", 2);
    pub const ARAMAIC: Self = Self(*b"arc", 3);
    pub const ARMENIAN: Self = Self(*b"hy ", 2);
    pub const ASSAMESE: Self = Self(*b"as ", 2);
    pub const ASTURIAN: Self = Self(*b"ast", 3);
    pub const ASU: Self = Self(*b"asa", 3);
    pub const ATSAM: Self = Self(*b"cch", 3);
    pub const AVESTAN: Self = Self(*b"ae ", 2);
    pub const AWADHI: Self = Self(*b"awa", 3);
    pub const AYMARA: Self = Self(*b"ay ", 2);
    pub const AZERBAIJANI: Self = Self(*b"az ", 2);
    pub const BAFIA: Self = Self(*b"ksf", 3);
    pub const BALINESE: Self = Self(*b"ban", 3);
    pub const BALUCHI: Self = Self(*b"bal", 3);
    pub const BAMBARA: Self = Self(*b"bm ", 2);
    pub const BANGLA: Self = Self(*b"bn ", 2);
    pub const BASAA: Self = Self(*b"bas", 3);
    pub const BASHKIR: Self = Self(*b"ba ", 2);
    pub const BASQUE: Self = Self(*b"eu ", 2);
    pub const BATAK_TOBA: Self = Self(*b"bbc", 3);
    pub const BAVARIAN: Self = Self(*b"bar", 3);
    pub const BELARUSIAN: Self = Self(*b"be ", 2);
    pub const BEMBA: Self = Self(*b"bem", 3);
    pub const BENA: Self = Self(*b"bez", 3);
    pub const BETAWI: Self = Self(*b"bew", 3);
    pub const BHOJPURI: Self = Self(*b"bho", 3);
    pub const BLIN: Self = Self(*b"byn", 3);
    pub const BODO: Self = Self(*b"brx", 3);
    pub const BOSNIAN: Self = Self(*b"bs ", 2);
    pub const BRETON: Self = Self(*b"br ", 2);
    pub const BULGARIAN: Self = Self(*b"bg ", 2);
    pub const BURIAT: Self = Self(*b"bua", 3);
    pub const BURMESE: Self = Self(*b"my ", 2);
    pub const CANTONESE: Self = Self(*b"yue", 3);
    pub const CARIAN: Self = Self(*b"xcr", 3);
    pub const CATALAN: Self = Self(*b"ca ", 2);
    pub const CEBUANO: Self = Self(*b"ceb", 3);
    pub const CENTRAL_ATLAS_TAMAZIGHT: Self = Self(*b"tzm", 3);
    pub const CENTRAL_KURDISH: Self = Self(*b"ckb", 3);
    pub const CHAKMA: Self = Self(*b"ccp", 3);
    pub const CHECHEN: Self = Self(*b"ce ", 2);
    pub const CHEROKEE: Self = Self(*b"chr", 3);
    pub const CHIGA: Self = Self(*b"cgg", 3);
    pub const CHINESE: Self = Self(*b"zh ", 2);
    pub const CHURCH_SLAVIC: Self = Self(*b"cu ", 2);
    pub const CHUVASH: Self = Self(*b"cv ", 2);
    pub const CLASSICAL_MANDAIC: Self = Self(*b"myz", 3);
    pub const COLOGNIAN: Self = Self(*b"ksh", 3);
    pub const COPTIC: Self = Self(*b"cop", 3);
    pub const CORNISH: Self = Self(*b"kw ", 2);
    pub const CORSICAN: Self = Self(*b"co ", 2);
    pub const CROATIAN: Self = Self(*b"hr ", 2);
    pub const CZECH: Self = Self(*b"cs ", 2);
    pub const DANISH: Self = Self(*b"da ", 2);
    pub const DIVEHI: Self = Self(*b"dv ", 2);
    pub const DOGRI: Self = Self(*b"doi", 3);
    pub const DUALA: Self = Self(*b"dua", 3);
    pub const DUTCH: Self = Self(*b"nl ", 2);
    pub const DZONGKHA: Self = Self(*b"dz ", 2);
    pub const EGYPTIAN_ARABIC: Self = Self(*b"arz", 3);
    pub const EMBU: Self = Self(*b"ebu", 3);
    pub const ENGLISH: Self = Self(*b"en ", 2);
    pub const ERZYA: Self = Self(*b"myv", 3);
    pub const ESPERANTO: Self = Self(*b"eo ", 2);
    pub const ESTONIAN: Self = Self(*b"et ", 2);
    pub const ETRUSCAN: Self = Self(*b"ett", 3);
    pub const EWE: Self = Self(*b"ee ", 2);
    pub const EWONDO: Self = Self(*b"ewo", 3);
    pub const FAROESE: Self = Self(*b"fo ", 2);
    pub const FILIPINO: Self = Self(*b"fil", 3);
    pub const FINNISH: Self = Self(*b"fi ", 2);
    pub const FRENCH: Self = Self(*b"fr ", 2);
    pub const FRIULIAN: Self = Self(*b"fur", 3);
    pub const FULAH: Self = Self(*b"ff ", 2);
    pub const GA: Self = Self(*b"gaa", 3);
    pub const GALICIAN: Self = Self(*b"gl ", 2);
    pub const GANDA: Self = Self(*b"lg ", 2);
    pub const GEEZ: Self = Self(*b"gez", 3);
    pub const GEORGIAN: Self = Self(*b"ka ", 2);
    pub const GERMAN: Self = Self(*b"de ", 2);
    pub const GOTHIC: Self = Self(*b"got", 3);
    pub const GREEK: Self = Self(*b"el ", 2);
    pub const GUARANI: Self = Self(*b"gn ", 2);
    pub const GUJARATI: Self = Self(*b"gu ", 2);
    pub const GUSII: Self = Self(*b"guz", 3);
    pub const HARYANVI: Self = Self(*b"bgc", 3);
    pub const HAUSA: Self = Self(*b"ha ", 2);
    pub const HAWAIIAN: Self = Self(*b"haw", 3);
    pub const HEBREW: Self = Self(*b"he ", 2);
    pub const HINDI: Self = Self(*b"hi ", 2);
    pub const HMONG_NJUA: Self = Self(*b"hnj", 3);
    pub const HUNGARIAN: Self = Self(*b"hu ", 2);
    pub const ICELANDIC: Self = Self(*b"is ", 2);
    pub const IGBO: Self = Self(*b"ig ", 2);
    pub const INARI_SAMI: Self = Self(*b"smn", 3);
    pub const INDONESIAN: Self = Self(*b"id ", 2);
    pub const INGUSH: Self = Self(*b"inh", 3);
    pub const INTERLINGUA: Self = Self(*b"ia ", 2);
    pub const INTERSLAVIC: Self = Self(*b"isv", 3);
    pub const INUKTITUT: Self = Self(*b"iu ", 2);
    pub const IRISH: Self = Self(*b"ga ", 2);
    pub const ITALIAN: Self = Self(*b"it ", 2);
    pub const JAPANESE: Self = Self(*b"ja ", 2);
    pub const JAVANESE: Self = Self(*b"jv ", 2);
    pub const JJU: Self = Self(*b"kaj", 3);
    pub const JOLA_FONYI: Self = Self(*b"dyo", 3);
    pub const KABUVERDIANU: Self = Self(*b"kea", 3);
    pub const KABYLE: Self = Self(*b"kab", 3);
    pub const KAINGANG: Self = Self(*b"kgp", 3);
    pub const KAKO: Self = Self(*b"kkj", 3);
    pub const KALAALLISUT: Self = Self(*b"kl ", 2);
    pub const KALENJIN: Self = Self(*b"kln", 3);
    pub const KAMBA: Self = Self(*b"kam", 3);
    pub const KANGRI: Self = Self(*b"xnr", 3);
    pub const KANNADA: Self = Self(*b"kn ", 2);
    pub const KASHMIRI: Self = Self(*b"ks ", 2);
    pub const KAZAKH: Self = Self(*b"kk ", 2);
    pub const KHMER: Self = Self(*b"km ", 2);
    pub const KIKUYU: Self = Self(*b"ki ", 2);
    pub const KINYARWANDA: Self = Self(*b"rw ", 2);
    pub const KOMI: Self = Self(*b"kv ", 2);
    pub const KONKANI: Self = Self(*b"kok", 3);
    pub const KOREAN: Self = Self(*b"ko ", 2);
    pub const KOYRABORO_SENNI: Self = Self(*b"ses", 3);
    pub const KOYRA_CHIINI: Self = Self(*b"khq", 3);
    pub const KURDISH: Self = Self(*b"ku ", 2);
    pub const KWASIO: Self = Self(*b"nmg", 3);
    pub const KYRGYZ: Self = Self(*b"ky ", 2);
    pub const LADINO: Self = Self(*b"lad", 3);
    pub const LAKOTA: Self = Self(*b"lkt", 3);
    pub const LANGI: Self = Self(*b"lag", 3);
    pub const LAO: Self = Self(*b"lo ", 2);
    pub const LATIN: Self = Self(*b"la ", 2);
    pub const LATVIAN: Self = Self(*b"lv ", 2);
    pub const LEPCHA: Self = Self(*b"lep", 3);
    pub const LIGURIAN: Self = Self(*b"lij", 3);
    pub const LIMBU: Self = Self(*b"lif", 3);
    pub const LINEAR_A: Self = Self(*b"lab", 3);
    pub const LINGALA: Self = Self(*b"ln ", 2);
    pub const LITHUANIAN: Self = Self(*b"lt ", 2);
    pub const LOMBARD: Self = Self(*b"lmo", 3);
    pub const LOWER_SORBIAN: Self = Self(*b"dsb", 3);
    pub const LOW_GERMAN: Self = Self(*b"nds", 3);
    pub const LUBA_KATANGA: Self = Self(*b"lu ", 2);
    pub const LUO: Self = Self(*b"luo", 3);
    pub const LUXEMBOURGISH: Self = Self(*b"lb ", 2);
    pub const LUYIA: Self = Self(*b"luy", 3);
    pub const LYCIAN: Self = Self(*b"xlc", 3);
    pub const LYDIAN: Self = Self(*b"xld", 3);
    pub const LU: Self = Self(*b"khb", 3);
    pub const MACEDONIAN: Self = Self(*b"mk ", 2);
    pub const MACHAME: Self = Self(*b"jmc", 3);
    pub const MAITHILI: Self = Self(*b"mai", 3);
    pub const MAKASAR: Self = Self(*b"mak", 3);
    pub const MAKHUWA_MEETTO: Self = Self(*b"mgh", 3);
    pub const MAKHUWA: Self = Self(*b"vmw", 3);
    pub const MAKONDE: Self = Self(*b"kde", 3);
    pub const MALAGASY: Self = Self(*b"mg ", 2);
    pub const MALAY: Self = Self(*b"ms ", 2);
    pub const MALAYALAM: Self = Self(*b"ml ", 2);
    pub const MALTESE: Self = Self(*b"mt ", 2);
    pub const MANIPURI: Self = Self(*b"mni", 3);
    pub const MANX: Self = Self(*b"gv ", 2);
    pub const MARATHI: Self = Self(*b"mr ", 2);
    pub const MASAI: Self = Self(*b"mas", 3);
    pub const MAZANDERANI: Self = Self(*b"mzn", 3);
    pub const MERU: Self = Self(*b"mer", 3);
    pub const METAʼ: Self = Self(*b"mgo", 3);
    pub const MONGOLIAN: Self = Self(*b"mn ", 2);
    pub const MORISYEN: Self = Self(*b"mfe", 3);
    pub const MUNDANG: Self = Self(*b"mua", 3);
    pub const MUSCOGEE: Self = Self(*b"mus", 3);
    pub const MAORI: Self = Self(*b"mi ", 2);
    pub const NAMA: Self = Self(*b"naq", 3);
    pub const NAVAJO: Self = Self(*b"nv ", 2);
    pub const NEPALI: Self = Self(*b"ne ", 2);
    pub const NEWARI: Self = Self(*b"new", 3);
    pub const NGIEMBOON: Self = Self(*b"nnh", 3);
    pub const NGOMBA: Self = Self(*b"jgo", 3);
    pub const NHEENGATU: Self = Self(*b"yrl", 3);
    pub const NIGERIAN_PIDGIN: Self = Self(*b"pcm", 3);
    pub const NORTHERN_FRISIAN: Self = Self(*b"frr", 3);
    pub const NORTHERN_KURDISH: Self = Self(*b"kmr", 3);
    pub const NORTHERN_LURI: Self = Self(*b"lrc", 3);
    pub const NORTHERN_SAMI: Self = Self(*b"se ", 2);
    pub const NORTHERN_SOTHO: Self = Self(*b"nso", 3);
    pub const NORTH_NDEBELE: Self = Self(*b"nd ", 2);
    pub const NORWEGIAN: Self = Self(*b"no ", 2);
    pub const NORWEGIAN_BOKMAL: Self = Self(*b"nb ", 2);
    pub const NORWEGIAN_NYNORSK: Self = Self(*b"nn ", 2);
    pub const NUER: Self = Self(*b"nus", 3);
    pub const NYANJA: Self = Self(*b"ny ", 2);
    pub const NYANKOLE: Self = Self(*b"nyn", 3);
    pub const NKO: Self = Self(*b"nqo", 3);
    pub const OCCITAN: Self = Self(*b"oc ", 2);
    pub const ODIA: Self = Self(*b"or ", 2);
    pub const OLD_IRISH: Self = Self(*b"sga", 3);
    pub const OLD_NORSE: Self = Self(*b"non", 3);
    pub const OLD_PERSIAN: Self = Self(*b"peo", 3);
    pub const OLD_UIGHUR: Self = Self(*b"oui", 3);
    pub const OROMO: Self = Self(*b"om ", 2);
    pub const OSAGE: Self = Self(*b"osa", 3);
    pub const OSSETIC: Self = Self(*b"os ", 2);
    pub const PAPIAMENTO: Self = Self(*b"pap", 3);
    pub const PASHTO: Self = Self(*b"ps ", 2);
    pub const PERSIAN: Self = Self(*b"fa ", 2);
    pub const PHOENICIAN: Self = Self(*b"phn", 3);
    pub const PIEDMONTESE: Self = Self(*b"pms", 3);
    pub const POLISH: Self = Self(*b"pl ", 2);
    pub const PORTUGUESE: Self = Self(*b"pt ", 2);
    pub const PRUSSIAN: Self = Self(*b"prg", 3);
    pub const PUNJABI: Self = Self(*b"pa ", 2);
    pub const QUECHUA: Self = Self(*b"qu ", 2);
    pub const RAJASTHANI: Self = Self(*b"raj", 3);
    pub const ROMANIAN: Self = Self(*b"ro ", 2);
    pub const ROMANSH: Self = Self(*b"rm ", 2);
    pub const ROMBO: Self = Self(*b"rof", 3);
    pub const RUNDI: Self = Self(*b"rn ", 2);
    pub const RUSSIAN: Self = Self(*b"ru ", 2);
    pub const RWA: Self = Self(*b"rwk", 3);
    pub const SABAEAN: Self = Self(*b"xsa", 3);
    pub const SAHO: Self = Self(*b"ssy", 3);
    pub const SAKHA: Self = Self(*b"sah", 3);
    pub const SAMARITAN: Self = Self(*b"smp", 3);
    pub const SAMBURU: Self = Self(*b"saq", 3);
    pub const SANGO: Self = Self(*b"sg ", 2);
    pub const SANGU: Self = Self(*b"sbp", 3);
    pub const SANSKRIT: Self = Self(*b"sa ", 2);
    pub const SANTALI: Self = Self(*b"sat", 3);
    pub const SARAIKI: Self = Self(*b"skr", 3);
    pub const SARDINIAN: Self = Self(*b"sc ", 2);
    pub const SCOTTISH_GAELIC: Self = Self(*b"gd ", 2);
    pub const SENA: Self = Self(*b"seh", 3);
    pub const SERBIAN: Self = Self(*b"sr ", 2);
    pub const SHAMBALA: Self = Self(*b"ksb", 3);
    pub const SHONA: Self = Self(*b"sn ", 2);
    pub const SICHUAN_YI: Self = Self(*b"ii ", 2);
    pub const SICILIAN: Self = Self(*b"scn", 3);
    pub const SILESIAN: Self = Self(*b"szl", 3);
    pub const SINDHI: Self = Self(*b"sd ", 2);
    pub const SINHALA: Self = Self(*b"si ", 2);
    pub const SINTE_ROMANI: Self = Self(*b"rmo", 3);
    pub const SLOVAK: Self = Self(*b"sk ", 2);
    pub const SLOVENIAN: Self = Self(*b"sl ", 2);
    pub const SOGA: Self = Self(*b"xog", 3);
    pub const SOMALI: Self = Self(*b"so ", 2);
    pub const SOUTHERN_ALTAI: Self = Self(*b"alt", 3);
    pub const SOUTHERN_SOTHO: Self = Self(*b"st ", 2);
    pub const SOUTH_NDEBELE: Self = Self(*b"nr ", 2);
    pub const SPANISH: Self = Self(*b"es ", 2);
    pub const STANDARD_MOROCCAN_TAMAZIGHT: Self = Self(*b"zgh", 3);
    pub const SUNDANESE: Self = Self(*b"su ", 2);
    pub const SWAHILI: Self = Self(*b"sw ", 2);
    pub const SWATI: Self = Self(*b"ss ", 2);
    pub const SWEDISH: Self = Self(*b"sv ", 2);
    pub const SWISS_GERMAN: Self = Self(*b"gsw", 3);
    pub const SYRIAC: Self = Self(*b"syr", 3);
    pub const TACHELHIT: Self = Self(*b"shi", 3);
    pub const TAITA: Self = Self(*b"dav", 3);
    pub const TAI_NUA: Self = Self(*b"tdd", 3);
    pub const TAJIK: Self = Self(*b"tg ", 2);
    pub const TAMIL: Self = Self(*b"ta ", 2);
    pub const TANGUT: Self = Self(*b"txg", 3);
    pub const TAROKO: Self = Self(*b"trv", 3);
    pub const TASAWAQ: Self = Self(*b"twq", 3);
    pub const TATAR: Self = Self(*b"tt ", 2);
    pub const TELUGU: Self = Self(*b"te ", 2);
    pub const TESO: Self = Self(*b"teo", 3);
    pub const THAI: Self = Self(*b"th ", 2);
    pub const TIBETAN: Self = Self(*b"bo ", 2);
    pub const TIGRE: Self = Self(*b"tig", 3);
    pub const TIGRINYA: Self = Self(*b"ti ", 2);
    pub const TOK_PISIN: Self = Self(*b"tpi", 3);
    pub const TONGAN: Self = Self(*b"to ", 2);
    pub const TSONGA: Self = Self(*b"ts ", 2);
    pub const TSWANA: Self = Self(*b"tn ", 2);
    pub const TURKISH: Self = Self(*b"tr ", 2);
    pub const TURKMEN: Self = Self(*b"tk ", 2);
    pub const TYAP: Self = Self(*b"kcg", 3);
    pub const UGARITIC: Self = Self(*b"uga", 3);
    pub const UKRAINIAN: Self = Self(*b"uk ", 2);
    pub const UNKNOWN_LANGUAGE: Self = Self(*b"und", 3);
    pub const UPPER_SORBIAN: Self = Self(*b"hsb", 3);
    pub const URDU: Self = Self(*b"ur ", 2);
    pub const UYGHUR: Self = Self(*b"ug ", 2);
    pub const UZBEK: Self = Self(*b"uz ", 2);
    pub const VAI: Self = Self(*b"vai", 3);
    pub const VENDA: Self = Self(*b"ve ", 2);
    pub const VENETIAN: Self = Self(*b"vec", 3);
    pub const VIETNAMESE: Self = Self(*b"vi ", 2);
    pub const VOLAPUK: Self = Self(*b"vo ", 2);
    pub const VUNJO: Self = Self(*b"vun", 3);
    pub const WALSER: Self = Self(*b"wae", 3);
    pub const WARAY: Self = Self(*b"war", 3);
    pub const WELSH: Self = Self(*b"cy ", 2);
    pub const WESTERN_FRISIAN: Self = Self(*b"fy ", 2);
    pub const WOLAYTTA: Self = Self(*b"wal", 3);
    pub const WOLOF: Self = Self(*b"wo ", 2);
    pub const XHOSA: Self = Self(*b"xh ", 2);
    pub const YANGBEN: Self = Self(*b"yav", 3);
    pub const YIDDISH: Self = Self(*b"yi ", 2);
    pub const YORUBA: Self = Self(*b"yo ", 2);
    pub const ZARMA: Self = Self(*b"dje", 3);
    pub const ZHUANG: Self = Self(*b"za ", 2);
    pub const ZULU: Self = Self(*b"zu ", 2);

    /// Return the language code as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// The default direction for the language.
    pub fn dir(self) -> Dir {
        match self.as_str() {
            "ar" | "dv" | "fa" | "he" | "ks" | "pa" | "ps" | "sd" | "ug" | "ur"
            | "yi" => Dir::RTL,
            _ => Dir::LTR,
        }
    }
}

impl FromStr for Lang {
    type Err = &'static str;

    /// Construct a language from a two- or three-byte ISO 639-1/2/3 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 2..=3) && iso.is_ascii() {
            let mut bytes = [b' '; 3];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected two or three letter language code (ISO 639-1/2/3)")
        }
    }
}

cast! {
    Lang,
    self => self.as_str().into_value(),
    string: EcoString => {
        let result = Self::from_str(&string);
        if result.is_err()
            && let Some((lang, region)) = string.split_once('-')
                && Lang::from_str(lang).is_ok() && Region::from_str(region).is_ok() {
                    return result
                        .hint(eco_format!(
                            "you should leave only \"{}\" in the `lang` parameter and specify \"{}\" in the `region` parameter",
                            lang, region,
                        ));
                }

        result?
    }
}

/// An identifier for a region somewhere in the world.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Region([u8; 2]);

impl Region {
    /// Return the region code as an all uppercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl PartialEq<&str> for Region {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl FromStr for Region {
    type Err = &'static str;

    /// Construct a region from its two-byte ISO 3166-1 alpha-2 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        if iso.len() == 2 && iso.is_ascii() {
            let mut bytes: [u8; 2] = iso.as_bytes().try_into().unwrap();
            bytes.make_ascii_uppercase();
            Ok(Self(bytes))
        } else {
            Err("expected two letter region code (ISO 3166-1 alpha-2)")
        }
    }
}

cast! {
    Region,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// An ISO 15924-type script identifier.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct WritingScript([u8; 4], u8);

impl WritingScript {
    /// Return the script as an all lowercase string slice.
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0[..usize::from(self.1)]).unwrap_or_default()
    }

    /// Return the description of the script as raw bytes.
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.0
    }
}

impl FromStr for WritingScript {
    type Err = &'static str;

    /// Construct a region from its ISO 15924 code.
    fn from_str(iso: &str) -> Result<Self, Self::Err> {
        let len = iso.len();
        if matches!(len, 3..=4) && iso.is_ascii() {
            let mut bytes = [b' '; 4];
            bytes[..len].copy_from_slice(iso.as_bytes());
            bytes.make_ascii_lowercase();
            Ok(Self(bytes, len as u8))
        } else {
            Err("expected three or four letter script code (ISO 15924 or 'math')")
        }
    }
}

cast! {
    WritingScript,
    self => self.as_str().into_value(),
    string: EcoString => Self::from_str(&string)?,
}

/// The name with which an element is referenced.
pub trait LocalName {
    /// The key of an element in order to get its localized name.
    const KEY: &'static str;

    /// Get the name in the given language and (optionally) region.
    fn local_name(lang: Lang, region: Option<Region>) -> &'static str {
        localized_str(lang, region, Self::KEY)
    }

    /// Gets the local name from the style chain.
    fn local_name_in(styles: StyleChain) -> &'static str
    where
        Self: Sized,
    {
        Self::local_name(styles.get(TextElem::lang), styles.get(TextElem::region))
    }
}

/// Retrieves the localized string for a given language and region.
/// Silently falls back to English if no fitting string exists for
/// the given language + region. Panics if no fitting string exists
/// in both given language + region and English.
#[comemo::memoize]
pub fn localized_str(lang: Lang, region: Option<Region>, key: &str) -> &'static str {
    let lang_region_bundle = parse_language_bundle(lang, region).unwrap();
    if let Some(str) = lang_region_bundle.get(key) {
        return str;
    }
    let lang_bundle = parse_language_bundle(lang, None).unwrap();
    if let Some(str) = lang_bundle.get(key) {
        return str;
    }
    let english_bundle = parse_language_bundle(Lang::ENGLISH, None).unwrap();
    english_bundle.get(key).unwrap()
}

/// Parses the translation file for a given language and region.
/// Only returns an error if the language file is malformed.
#[comemo::memoize]
fn parse_language_bundle(
    lang: Lang,
    region: Option<Region>,
) -> Result<FxHashMap<&'static str, &'static str>, &'static str> {
    let language_tuple = TRANSLATIONS.iter().find(|it| it.0 == lang_str(lang, region));
    let Some((_lang_name, language_file)) = language_tuple else {
        return Ok(FxHashMap::default());
    };

    let mut bundle = FxHashMap::default();
    let lines = language_file.trim().lines();
    for line in lines {
        if line.trim().starts_with('#') {
            continue;
        }
        let (key, val) = line
            .split_once('=')
            .ok_or("malformed translation file: line without \"=\"")?;
        let (key, val) = (key.trim(), val.trim());
        if val.is_empty() {
            return Err("malformed translation file: empty translation value");
        }
        let duplicate = bundle.insert(key.trim(), val.trim());
        if duplicate.is_some() {
            return Err("malformed translation file: duplicate key");
        }
    }
    Ok(bundle)
}

/// Convert language + region to a string to be able to get a file name.
fn lang_str(lang: Lang, region: Option<Region>) -> EcoString {
    EcoString::from(lang.as_str())
        + region.map_or_else(EcoString::new, |r| EcoString::from("-") + r.as_str())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use rustc_hash::FxHashSet;
    use typst_utils::option_eq;

    use super::*;

    fn translation_files_iter() -> impl Iterator<Item = PathBuf> {
        std::fs::read_dir("translations")
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|e| e.is_file() && e.extension().is_some_and(|e| e == "txt"))
    }

    #[test]
    fn test_region_option_eq() {
        let region = Some(Region([b'U', b'S']));
        assert!(option_eq(region, "US"));
        assert!(!option_eq(region, "AB"));
    }

    #[test]
    fn test_all_translations_included() {
        let defined_keys =
            FxHashSet::<&str>::from_iter(TRANSLATIONS.iter().map(|(lang, _)| *lang));
        let mut checked = 0;
        for file in translation_files_iter() {
            assert!(
                defined_keys.contains(
                    file.file_stem()
                        .expect("translation file should have basename")
                        .to_str()
                        .expect("translation file name should be utf-8 encoded")
                ),
                "translation from {:?} should be registered in TRANSLATIONS in {}",
                file.file_name().unwrap(),
                file!(),
            );
            checked += 1;
        }
        assert_eq!(TRANSLATIONS.len(), checked);
    }

    #[test]
    fn test_all_translation_files_formatted() {
        for file in translation_files_iter() {
            let content = std::fs::read_to_string(&file)
                .expect("translation file should be in utf-8 encoding");
            let filename = file.file_name().unwrap();
            assert!(
                content.ends_with('\n'),
                "translation file {filename:?} should end with linebreak",
            );
            for line in content.lines() {
                assert_eq!(
                    line.trim(),
                    line,
                    "line {line:?} in {filename:?} should not have extra whitespaces"
                );
            }
        }
    }

    #[test]
    fn test_translations_sorted() {
        assert!(
            TRANSLATIONS.is_sorted_by_key(|(lang, _)| lang),
            "TRANSLATIONS should be sorted"
        );
    }
}
