use cantrip_api::*;

#[test]
fn corrected_waiting_texts_preserve_which_source_is_named() {
    let table = CardTable::new(cantrip_cards::all());
    let name = "Goblin Kites";
    let card = table.find(name).unwrap();
    assert!(!table.names_its_source(Waiting::Delayed(AbilityRef::delayed(card, 0))), "{name} refers to the affected creature, not its own source");
}
