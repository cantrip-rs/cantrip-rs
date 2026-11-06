use cantrip_api::*;

#[test]
fn corrected_waiting_texts_preserve_which_source_is_named() {
    let table = CardTable::new(cantrip_cards::all());
    let snowman = table.find("Goblin Snowman").unwrap();
    assert!(table.names_its_source(Waiting::Prevention(snowman, 0)));
}
