#[test]
fn chandra_emblem_has_no_mana_cost() {
    let emblem = cantrip_cards::files()
        .into_iter()
        .find(|(card, _)| card.name == "Chandra, Awakened Inferno")
        .expect("Chandra card")
        .1
        .iter()
        .find(|def| def.name == "Chandra, Awakened Inferno emblem")
        .expect("Chandra emblem");
    assert!(!emblem.has_cost, "an emblem has no mana cost");
}
