#[test]
fn wolves_have_no_mana_cost_and_show_their_banding_ability() {
    let cards = cantrip_cards::all();
    let wolf = cards.iter().find(|card| card.name == "Wolves of the Hunt").unwrap();
    assert!(!wolf.has_cost);
    assert_eq!(wolf.text, "Bands with other creatures named Wolves of the Hunt.");
    assert_eq!(wolf.abilities[0].text(), wolf.text);
}
