//! Agatha of the Vile Cauldron's "cost {X} less to activate, where X is Agatha's power" takes her whole power off, past what a
//! byte holds (RX-5: cost changes are 32-bit). At power 300, Fiend Artisan's {X}{B/G} with X = 300 costs just {B/G}.
use cantrip_api::*;
use cantrip_engine::{Game, Setup};
use cantrip_mechanics::statics::{plus_pt, your_creatures};

static PUMP: CardDef = CardDef {
    name: "Probe Agatha pump",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtModify, your_creatures, plus_pt::<299, 0>, "Creatures you control get +299/+0.")],
    ..CardDef::BLANK
};

#[test]
fn agatha_at_power_300_takes_300_off_a_creatures_ability() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().iter().copied().chain([&PUMP]))));
    let mut game = Game::new(table, &Setup { decks: vec![vec![], vec![]], ..Default::default() });
    let you = PlayerId(0);
    let mut put = |name: &str| {
        let id = game.state.push_obj(Obj::new(table.find(name).unwrap(), you, Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    let agatha = put("Agatha of the Vile Cauldron");
    let artisan = put("Fiend Artisan");
    put("Probe Agatha pump");
    game.state.see_statics();
    assert_eq!(game.chars(agatha).power, Some(300));
    let def = table.get(table.find("Agatha of the Vile Cauldron").unwrap());
    let ability = AbilityRef { card: table.find("Fiend Artisan").unwrap(), index: 0 };
    let delta = (def.cost_changes[0])(&game, agatha, you, Costed::Ability(artisan, ability, None), &[]);
    assert_eq!(delta, CostDelta::less_to_one(300), "her whole power comes off, none of it cut to 255");
}
