use super::*;

const NAMES: &[&str] = &[
    "(25-09-16) Plans [c004]_Maywood-Ave-Upland [Mousa] pool spa.pdf",
    "(25-09-20) Plans [c004]_Maywood-Ave-Upland [Mousa] pool spa (3).pdf",
    "(25-10-29) Plans [c004]_DOC_17848 Otilla St, Bloomington[McGreggor] Pool [Recovered].pdf",
    "(25-11-10) Pool Plans [c004]_DOC 17848 bloomington [McGreggor].pdf",
    "(26-07-10) Plans [c004]_DOC_23857 Red Clover Cir Murrieta [~] Pool.pdf",
    "Tarea Calculo Integrales final.pdf",
    "Tarea Calculo Integrales.pdf",
    "Podcast Ep 01 raw.wav",
    "Podcast Ep 01 raw (2).wav",
    "Campaign Summer Ads.psd",
    "Campaign Winter Ads.psd",
    "Toki Promo Intro_v2_final.mp4",
    "Toki Promo Intro_v3.mp4",
    "notes.md",
    "api-refactor notes.md",
    "Intro final.mp4",
    "Fedex_01.pdf",
    "Fedex_02.pdf",
    "IMG_2041.jpg",
    "2026-03-14.mp4",
];

fn found() -> Vec<(Vec<&'static str>, Group)> {
    let names: Vec<String> = NAMES.iter().map(|n| n.to_string()).collect();
    group(&names).into_iter().map(|g| (g.members.iter().map(|&m| NAMES[m]).collect(), g)).collect()
}

fn group_with(name: &str) -> Option<Group> {
    found().into_iter().find(|(files, _)| files.contains(&name)).map(|(_, g)| g)
}

#[test]
fn versions_of_one_name_become_one_project() {
    let g = group_with("(25-09-16) Plans [c004]_Maywood-Ave-Upland [Mousa] pool spa.pdf").unwrap();
    assert_eq!(g.members.len(), 2);
    assert_eq!(g.tier, Tier::Sure);
    assert!(g.title.contains("Maywood"), "{}", g.title);
}

#[test]
fn a_long_number_links_names_that_share_few_words() {
    let g = group_with("(25-11-10) Pool Plans [c004]_DOC 17848 bloomington [McGreggor].pdf").unwrap();
    assert_eq!(g.members.len(), 2);
    assert_eq!(g.owner.as_deref(), Some("McGreggor"));
    assert_eq!(g.created.as_deref(), Some("25-10-29"));
}

#[test]
fn a_label_on_many_names_is_never_the_owner() {
    let g = group_with("(25-09-16) Plans [c004]_Maywood-Ave-Upland [Mousa] pool spa.pdf").unwrap();
    assert_eq!(g.owner.as_deref(), Some("Mousa"));
}

#[test]
fn two_real_owners_are_reported_instead_of_picked() {
    let names = vec!["Alpha Beta Gamma [Ana].pdf".to_string(), "Alpha Beta Gamma [Luis].pdf".to_string()];
    let groups = group(&names);
    assert_eq!(groups[0].owner, None);
    assert!(groups[0].owner_conflict);
}

#[test]
fn names_of_only_one_generic_word_stay_loose() {
    for name in ["notes.md", "Intro final.mp4", "Fedex_01.pdf", "IMG_2041.jpg", "2026-03-14.mp4"] {
        assert!(group_with(name).is_none(), "{name}");
    }
}

#[test]
fn names_that_differ_by_one_word_wait_for_the_user() {
    let g = group_with("Campaign Summer Ads.psd").unwrap();
    assert_eq!(g.tier, Tier::Likely);
}

#[test]
fn a_series_stays_together() {
    assert_eq!(group_with("Podcast Ep 01 raw.wav").unwrap().members.len(), 2);
}

#[test]
fn accents_and_typos_do_not_split_a_project() {
    let names = vec!["Ensayo Revolución Francesa.docx".to_string(), "ensayo revolucion francesa final.docx".to_string(), "Ensayo Revolucion Francessa.docx".to_string()];
    assert_eq!(group(&names)[0].members.len(), 3);
}

#[test]
fn nothing_groups_in_an_empty_list() {
    assert!(group(&[]).is_empty());
}
