use bencher::Bencher;
use typeset::font::{*, FontClass::*};
use typeset::style::TextStyle;


fn font_loading(b: &mut Bencher) {
    let provider = FileSystemFontProvider::from_listing("../fonts/fonts.toml").unwrap();
    let providers = vec![Box::new(provider) as Box<dyn FontProvider>];
    let font_loader = FontLoader::new(&providers);

    let text = include_str!("../test/shakespeare.tps");

    let mut style = TextStyle {
        classes: vec![Regular],
        fallback: vec![
            Family("Helvetica".to_string()),
            Family("Computer Modern".to_string()),
            Serif,
            Monospace,
        ],
        font_size: 12.0,
        line_spacing: 1.0,
        paragraph_spacing: 1.0,
    };

    b.iter(|| {
        for character in text.chars() {
            match character {
                '_' => style.toggle_class(Italic),
                '*' => style.toggle_class(Bold),
                '\n' => {},
                _ => {
                    let _font = font_loader.get(FontQuery {
                        character,
                        classes: style.classes.clone(),
                        fallback: style.fallback.clone(),
                    }).unwrap();
                },
            }
        }
    });
}

bencher::benchmark_group!(benches, font_loading);
bencher::benchmark_main!(benches);
