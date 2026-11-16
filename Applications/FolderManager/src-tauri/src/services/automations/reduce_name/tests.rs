use proptest::prelude::*;

use super::*;

fn set(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_lowercase()).collect()
}

fn is_dated(name: &str) -> bool {
    let b = name.as_bytes();
    name.starts_with('(') && name.find(')').is_some_and(|e| (9..=15).contains(&e) && b[1..e].iter().all(|c| c.is_ascii_digit() || matches!(c, b'-' | b'_')))
}

#[test]
fn new_name_table() {
    for (input, want) in [
        ("Photo (1).png", "Photo (1).png"),
        ("image.png", "d2b5.png"),
        ("image (3).png", "da6e.png"),
        ("Ganyu.600.3545662.jpg", "Ganyu_3545.jpg"),
        ("a3f9c2d1-1111-2222-3333-444455556666.png", "a3f9.png"),
        ("Sakuya_9f8a7b6c5d4e3f2a1b0c.png", "Sakuya_9f8a.png"),
        ("sample-abcdef0123456789abcd.mp4", "abcd.mp4"),
        ("sample__ABCDEFG1234567890hij.png", "ABCDEFG1234hij.png"),
        ("thumb.jpeg.13bc.jpg", "thumb.13bc.jpg"),
        ("6298292_jpg.jpg", "6298292.jpg"),
        ("Bob  --  Ross__final.txt", "Bob-Ross_final.txt"),
        ("reddit-v0-abc123def.jpg", "reddit.jpg"),
        ("title_n.png", "title.png"),
        ("[Mike] (26-05-04) cosa.png", "[Mike] (26-05-04) cosa.png"),
        ("日本語_ファイル.txt", "9bda.txt"),
        ("Ünïcode Név 12345678901234.pdf", "Ünïcode Név 1234.pdf"),
        ("20240905_foto.jpg", "(24-09-05)_foto.jpg"),
        ("IMG_20240905_121359.jpg", "(24-09-05_12-13).jpg"),
        ("27 sept 2026 nota.txt", "(26-09-27)_nota.txt"),
        ("Mar 28, 2025 at 3.45 PM shot.png", "(25-03-28)_at 3.45 PM shot.png"),
        ("report_20241301.pdf", "report_2024.pdf"),
        ("12345678 foo.png", "1234 foo.png"),
        ("1234567890123456.jpg", "1234.jpg"),
        (".bashrc", ".bashrc"),
        ("name.", "name"),
        ("a.b.c.tar.gz", "a.b.c.tar.gz"),
        ("~$x.docx", "~$x.docx"),
    ] {
        assert_eq!(new_name(input), want, "{input}");
    }
}

#[test]
fn generic_names_hash_to_a_stable_short_name() {
    assert_eq!(new_name("image.png"), new_name("image.png"));
    assert_ne!(new_name("image.png"), new_name("image (3).png"));
    assert_eq!(md5::hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
    assert_eq!(md5::hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
}

#[test]
fn date_names_table() {
    for (input, date, rest) in [
        ("20240905_foto", "24-09-05", "foto"),
        ("IMG_20240905_121359", "24-09-05_12-13", ""),
        ("27 sept 2026 nota", "26-09-27", "nota"),
        ("Mar 28, 2025 at 3.45.10 PM shot", "25-03-28_15-45", "shot"),
        ("Mar 28, 2025 at 3.45 PM shot", "25-03-28", "at 3.45 PM shot"),
        ("trip 2024-02-29 beach", "24-02-29", "trip_beach"),
        ("PXL_20230101_120000", "23-01-01_12-00", ""),
    ] {
        let (d, r) = dated::split_date(input).unwrap_or_else(|| panic!("{input}"));
        assert_eq!((d.as_str(), r.as_str()), (date, rest), "{input}");
    }
    for stem in ["no date here", "20230230_x", "2024-13-01", "(24-09-05)_foto", "Mayhem 12 2024", "v2024"] {
        assert!(dated::split_date(stem).is_none(), "{stem}");
    }
    assert!(dated::has_date("(24-09-05_12-13)_x") && !dated::has_date("plain"));
}

#[test]
fn leap_days_are_only_valid_in_leap_years() {
    assert!(dated::split_date("a 2024-02-29").is_some());
    assert!(dated::split_date("a 2023-02-29").is_none());
    assert!(dated::split_date("a 2100-02-29").is_none());
}

#[test]
fn stamp_has_the_full_shape() {
    let s = dated::stamp(1_700_000_000);
    let b = s.as_bytes();
    assert!(s.len() == 14 && b[2] == b'-' && b[5] == b'-' && b[8] == b'_' && b[11] == b'-', "{s}");
    assert!(s.starts_with("23-11-1"), "{s}");
}

#[test]
fn dated_name_adds_moves_and_keeps_dates() {
    let stamp = dated::stamp(1_700_000_000);
    assert_eq!(dated_name("cat.png", Some(1_700_000_000)), format!("({stamp})_cat.png"));
    assert_eq!(dated_name("cat.png", None), "cat.png");
    assert_eq!(dated_name("foto 20240905.jpg", None), "(24-09-05)_foto.jpg");
    assert_eq!(dated_name("20240905.jpg", None), "(24-09-05).jpg");
    assert_eq!(dated_name("(24-09-05)_foto.jpg", Some(0)), "(24-09-05)_foto.jpg");
    assert_eq!(dated_name("noext", Some(1_700_000_000)), format!("({stamp})_noext"));
}

#[test]
fn undated_name_drops_the_date_but_never_empties_a_name() {
    for (input, want) in [
        ("20240905_foto.jpg", "foto.jpg"),
        ("(24-09-05)_foto.jpg", "foto.jpg"),
        ("(24-09-05_12-13)_foto.jpg", "foto.jpg"),
        ("(24-09-05).jpg", "(24-09-05).jpg"),
        ("20240905.jpg", "20240905.jpg"),
        ("cat.png", "cat.png"),
    ] {
        assert_eq!(undated_name(input), want, "{input}");
    }
}

#[test]
fn unbracketed_name_table() {
    for (input, want) in [
        ("Photo (1).png", "Photo 1.png"),
        ("[Mike] (26-05-04) cosa.png", "Mike 26-05-04 cosa.png"),
        ("{a}_[b].txt", "a_b.txt"),
        ("Bob  --  Ross__final.txt", "Bob-Ross_final.txt"),
        ("()", "()"),
        ("[].txt", "[].txt"),
        ("日本語 (改).txt", "日本語 改.txt"),
        ("plain.txt", "plain.txt"),
    ] {
        assert_eq!(unbracketed_name(input), want, "{input}");
    }
}

#[test]
fn unbracketed_name_keeps_hidden_files_hidden() {
    assert_eq!(unbracketed_name(".bashrc"), ".bashrc");
}

#[test]
fn unique_name_table() {
    assert_eq!(unique_name("a.txt", &set(&["b.txt"])), "a.txt");
    assert_eq!(unique_name("a.txt", &set(&["a.txt"])), "a (2).txt");
    assert_eq!(unique_name("A.TXT", &set(&["a.txt", "a (2).txt"])), "A (3).TXT");
    assert_eq!(unique_name("a (9).txt", &set(&["a (9).txt", "a.txt"])), "a (10).txt");
    assert_eq!(unique_name("noext", &set(&["noext"])), "noext (2)");
    assert_eq!(unique_name("a (1).txt", &set(&["a (1).txt"])), "a (2).txt");
    assert_eq!(unique_name("a (٣).txt", &set(&["a (٣).txt"])), "a (4).txt");
    assert_eq!(unique_name(".bashrc", &set(&[".bashrc"])), ".bashrc (2)");
    let huge = format!("a ({}).txt", "9".repeat(60));
    assert!(!set(&[&huge]).contains(&unique_name(&huge, &set(&[&huge]))));
}

const STEM: &str = "[A-Za-z0-9 _.()\\[\\]+~-]{0,23}[A-Za-z0-9]";
const EXT: &str = "(\\.(jpg|png|pdf|txt|tar\\.gz))?";

proptest! {
    #[test]
    fn reduce_is_idempotent(stem in STEM, ext in EXT) {
        let once = new_name(&format!("{stem}{ext}"));
        prop_assert_eq!(new_name(&once), once.clone());
    }

    #[test]
    fn reduce_never_returns_an_empty_name(stem in "\\PC{1,30}", ext in EXT) {
        let out = new_name(&format!("{stem}{ext}"));
        prop_assert!(!out.is_empty());
    }

    #[test]
    fn reduce_keeps_the_real_extension(stem in "[A-Za-z]{1,12}", ext in "(jpg|png|pdf|mp4)") {
        let out = new_name(&format!("{stem} 20240905 {stem}.{ext}"));
        let tail = format!(".{ext}");
        prop_assert!(out.ends_with(&tail));
    }

    #[test]
    fn undating_twice_is_undating_once(stem in STEM, ext in EXT) {
        let once = undated_name(&format!("{stem}{ext}"));
        prop_assert_eq!(undated_name(&once), once.clone());
    }

    #[test]
    fn unbracketing_twice_is_unbracketing_once(stem in "[A-Za-z0-9 _()\\[\\]{}-]{1,24}", ext in EXT) {
        let once = unbracketed_name(&format!("{stem}{ext}"));
        prop_assert_eq!(unbracketed_name(&once), once.clone());
    }

    #[test]
    fn adding_a_date_twice_is_adding_it_once(stem in "[A-Za-z]{1,12}", ext in EXT, secs in 0i64..4_000_000_000) {
        let once = dated_name(&format!("{stem}{ext}"), Some(secs));
        prop_assert!(is_dated(&once), "{once}");
        prop_assert_eq!(dated_name(&once, Some(secs)), once.clone());
    }

    #[test]
    fn unique_names_never_collide(stem in "[a-c]{1,2}", count in 0usize..6, ext in "(\\.txt)?") {
        let name = format!("{stem}{ext}");
        let mut taken = HashSet::new();
        for _ in 0..=count {
            let next = unique_name(&name, &taken);
            prop_assert!(!taken.contains(&next.to_lowercase()), "{next}");
            taken.insert(next.to_lowercase());
        }
    }
}
