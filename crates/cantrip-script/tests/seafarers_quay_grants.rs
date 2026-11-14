//! The Quay grants its ability only to blue legendary creatures its controller controls.
use cantrip_engine::*;
use std::sync::OnceLock;

#[test]
fn grants_only_eligible_creatures() {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    let cards = TABLE.get_or_init(|| CardTable::new(cantrip_cards::all()));
    let mut game = Game::new(cards, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let quay_card = cards.find("Seafarer's Quay").unwrap();
    let mut put = |name: &str, controller: PlayerId| {
        let card = cards.find(name).unwrap();
        let id = game.state.push_obj(Obj::new(card, controller, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    put("Seafarer's Quay", PlayerId(0));
    let blue_legend = put("Rona, Herald of Invasion", PlayerId(0));
    let other_blue_legend = put("Rona, Herald of Invasion", PlayerId(1));
    let blue_nonlegend = put("Merfolk of the Pearl Trident", PlayerId(0));
    let white_legend = put("Isamaru, Hound of Konda", PlayerId(0));
    game.state.see_statics();
    let granted = AbilityRef::delayed(quay_card, 0);
    assert!(game.chars(blue_legend).abilities.contains(&Some(granted)));
    for id in [other_blue_legend, blue_nonlegend, white_legend] {
        assert!(!game.chars(id).abilities.contains(&Some(granted)));
    }
}
