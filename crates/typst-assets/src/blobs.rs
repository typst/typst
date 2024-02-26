/// The list of all currently downloadable blobs.
///
/// Files will be downloaded from `BLOB_URL/{hash}`.
///
/// Important: Must be in alphabetic order (according to Rust's comparison
/// which just compares ASCII values, so uppercase comes before lowercase).
/// This is enforced through a unit test.
pub const BLOBS: &[(&str, &str)] = &[
    ("1-writing-app.png", "jC7CKXXkSQMW-TiPT4_3-6mRcfcwcpJX3NOybI4v6yU="),
    ("1-writing-upload.png", "HhtxNctlQFkJfQj-GNxxqBqBCjiCWpCKBOXSDbeQzuI="),
    ("2-formatting-autocomplete.png", "DA5jJihqOk8QiBkJgBiZSnO-CVxn23aeMhVXdo4phSU="),
    ("3-advanced-paper.png", "lgehAHpNW-QEOBvXIYZQps2BDpMK7xe3twYc3MZ-lkw="),
    ("3-advanced-team-settings.png", "sj7tXL3YwDx5s5n5Eu76dBaInirYCi6XGuUN8QKZQlU="),
    ("DejaVuSansMono-Bold.ttf", "vOYPG0QhrNnqUbpmI9cCTsvmgXqVPjZU32Kl5r3492k="),
    ("DejaVuSansMono-BoldOblique.ttf", "kXE6cdVQu6IsKmsrsqmtj5oVnhLk6fClsmd5mLohIT4="),
    ("DejaVuSansMono-Oblique.ttf", "dCCXhAxUGHDo1txcmze7HO7qbA3t0dR1-vkD7533NLA="),
    ("DejaVuSansMono.ttf", "tKbD5Pqrh3P0_3YdVkUWRkCfKavt1o8F04wt9mfTxYI="),
    ("FiraMath-Regular.otf", "ICjL091NjAzxYIUg60dZlWqDpnkx17bY58MTUgGG41s="),
    ("IBMPlexSans-Bold.ttf", "hWxB19R7unSxB-Um749Jlo-yo6EpzcPF71iZujwtwYE="),
    ("IBMPlexSans-Light.ttf", "Y3oBEYfF4UaS8C17CrJNaY_SiVu_4eqE_IgwRk7MOgw="),
    ("IBMPlexSans-Medium.ttf", "Ed3eiMKe9-UfXAPaf94oUIVGmHkTnQBvYxpi26m70Gk="),
    ("IBMPlexSans-Regular.ttf", "hS3vfiT3txurbopcmwKyA-RbDvWWl_6vEW5-gJGteio="),
    ("IBMPlexSansCondensed-Regular.ttf", "EsLK-dga7pwfNBopl8uBOg-9AyJzKWChp19T_dKABOI="),
    ("IBMPlexSansDevanagari-Regular.ttf", "Bi7Q5Tj0v5D5qeiEFbuRVv5cLiyfJvnOx2rgjWolUiM="),
    ("IBMPlexSerif-Regular.ttf", "KfG0-9pJB0dVO0d4D-sbc4MsSFgYlOvxyqmJWaAk_ck="),
    ("InriaSerif-Bold.ttf", "rAnP3o1SmyzWO9f0F1VqosQsoX9d9qAi-vkc_0NXBgk="),
    ("InriaSerif-BoldItalic.ttf", "18FAzbYaTB7coXq-0dnVf-WzRdAZXVgSnJjmHozDD5s="),
    ("InriaSerif-Italic.ttf", "mEIv17g83-P50j2r1g3vpW8340zTnX_OrxKbsgDR-1M="),
    ("InriaSerif-Regular.ttf", "kHwgDjiJm61WaNgbTA_T6IDX2gURQkBcPM17HC9F7bU="),
    ("LinLibertine_R.ttf", "BuK2eqmomecfeLkdohzQu6D7HFeR_TtYdS6iWkubedc="),
    ("LinLibertine_RB.ttf", "sETiIz208aeS6EIoJ2efO8O1c8FkWX6dOwOmPxI7TN8="),
    ("LinLibertine_RBI.ttf", "gwp4DDVfi1d8fTTUfMxF5amfIRep4e5KN-5bcd6OuNo="),
    ("LinLibertine_RI.ttf", "gAamIK50h7RcgnnMUbZ27AIJr3FR_qANad9EFCFx57Y="),
    ("NewCM10-Bold.otf", "RnWkKYH7VIkWr2T0Sprv1ug1EF1oP6dTjM9wI8uVvss="),
    ("NewCM10-BoldItalic.otf", "k1PUIV0EMnZmosUadIGlSuMKxg4NcGNGLqnLxFe0G8U="),
    ("NewCM10-Italic.otf", "ILnj-Vtk9qo2_1Dk9mKdBwQhMRxXrE3obZYRaPXlmS4="),
    ("NewCM10-Regular.otf", "xtLuVTuRIcqKK8MVEGLgNqM2ffSeyYocSUexK1XN7wY="),
    ("NewCMMath-Book.otf", "OcV73TZDOtU0SWLGmgpjXIuAjTq2_ZNmZWt-rfWpz0k="),
    ("NewCMMath-Regular.otf", "_dLcfNQ-dmJId0Q3qoAG9PWBH71-5V_tCmNMMt-MejE="),
    ("NotoColorEmoji.ttf", "vyqFBrgGFLoZCjTHsDevEmmn1hT-nzthPMFc3uxvgUs="),
    ("NotoSansArabic-Regular.ttf", "iOuLJul0djwZ35AmlLw8WB6kktz_j71m_6KbwammGAE="),
    ("NotoSansSymbols2-Regular.ttf", "iC0UK5oe8_1_pCJdvpXBD6tmZCButJZMj_cFpPbQKYg="),
    ("NotoSansThai-Regular.ttf", "G3-CX8B97ruqyGC94MoIJ7ee5uXHjn3Y8PdAqxHYZKE="),
    ("NotoSerifCJKjp-Regular.otf", "Y6xoANlGZddUlqUVNS-A9DbHjK_8OJq08w5ukMMCI20="),
    ("NotoSerifCJKkr-Regular.otf", "fSUMVC6P3xoLQDBd1nkUD3wn1uSKNZCRwrbaWESlaXM="),
    ("NotoSerifCJKsc-Bold.otf", "D-6x6rkLgTvl-0kzJLO-u-bvjY41ulYmAVh-X3om0FI="),
    ("NotoSerifCJKsc-Regular.otf", "nMuSIof0tKlBrOgjIe5uH_Y0DrU_HBOw5ihaEX0bAFc="),
    ("NotoSerifCJKtc-Bold.otf", "Tq6sT_VeCiPnCgQVSHDQdynf6gf47xlf3WzwI_4YQ2E="),
    ("NotoSerifCJKtc-Regular.otf", "m_7aPPcf9B69z-C2HKgoRWS40j_kWP8VHrhzkjbV3Xo="),
    ("NotoSerifHebrew-Bold.ttf", "GlEu_znc9D1ocC5PaXXX2K9eUlKUAjclXIC5XN5aBzA="),
    ("NotoSerifHebrew-Regular.ttf", "krFx47SKLSvDzOAr1hQ5WGY7L9SrD9t3z7KGVlFjU5g="),
    ("PTSans-Regular.ttf", "QZ4kAwPxGADCsNJLGb02GDG-F4kUJYauylvweKHncz4="),
    ("Roboto-Regular.ttf", "eX419_XWAgpcbqE7QuzWaLz7O7xLqg50dzUn5bbLMXQ="),
    ("TwitterColorEmoji.ttf", "EdRfWvxqahnwyZgxvEf4eeQVeoZ1z6pDZSn07yJ928s="),
    ("Ubuntu-Regular.ttf", "Zv6pwACR8l64pSZUgCO2FUeFh2qQCvLY9HKSJolpgWM="),
    ("glacier.jpg", "htLa5APfJkUFhhIynCBuYB3fltOoA32DEWYm7oLXoE0="),
    ("graph.png", "H8ZQJy-1dr2VjuaRMRubr6MAlQ47-ibKRsuBZqJ5Igc="),
    ("hello.wasm", "qPPmG9ausl5wRDjUqrzfavaLNgDePf0r_vY_pbqq32Y="),
    ("molecular.jpg", "EGrr8EzahRMvc8eUSdAFYjaQCiQWxx5F_DjfXvNbtY4="),
    ("plugin-oob.wasm", "I8Iw4KaMDUmJ1Q98OzrwmQJzfNFUWoTjF9U7IcgHnVo="),
    ("rhino.png", "GO40Soj7_8sxvwG3FxnbUtcxwdfo6qYEvbusD-NXNbE="),
    ("tiger.jpg", "fjKyaLi5N7OIS5imzYLeq7V8MDtiTCoFRt1mFfxWlH0="),
    ("typing.jpg", "cso16VsDnsiI26p4aGO9gEn856RyHK2IlmHQBuL88-M="),
];

#[cfg(test)]
mod tests {
    use super::BLOBS;

    #[test]
    fn test_blob_list_sorted() {
        for w in BLOBS.windows(2) {
            let l = &w[0].0;
            let r = &w[1].0;
            if l > r {
                panic!("list is not sorted: {l:?} is not <= {r:?}");
            }
        }
    }
}
