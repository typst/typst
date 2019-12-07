use typstc::syntax::*;

use Token::{
    Space as S, Newline as N, LeftBracket as LB,
    RightBracket as RB, Text as T, *
};

macro_rules! tokens {
    ($($src:expr =>($line:expr)=> $tokens:expr)*) => ({
        #[allow(unused_mut)]
        let mut cases = Vec::new();
        $(cases.push(($line, $src, $tokens.to_vec()));)*
        cases
    });
}

fn main() {
    let tests = include!("cache/parsing.rs");

    let mut errors = false;
    for (file, cases) in tests.into_iter() {
        print!("Testing: {}. ", file);

        let mut okay = 0;
        let mut failed = 0;

        for (line, src, expected) in cases.into_iter() {
            let found: Vec<_> = tokenize(src).map(Spanned::value).collect();

            if found == expected {
                okay += 1;
            } else {
                if failed == 0 {
                    println!();
                }

                println!(" - Case failed in file {}.rs in line {}.", file, line);
                println!("   - Source:   {:?}", src);
                println!("   - Expected: {:?}", expected);
                println!("   - Found:    {:?}", found);

                failed += 1;
                errors = true;
            }
        }

        print!("{} okay, {} failed.", okay, failed);
        if failed == 0 {
            print!(" ✔")
        }
        println!();
    }

    println!();

    if errors {
        std::process::exit(-1);
    }
}
