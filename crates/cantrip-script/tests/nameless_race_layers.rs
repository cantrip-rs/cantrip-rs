//! Nameless Race's characteristic-defining ability applies before base-setting effects (CR 613.4a–b).
use cantrip_engine::*;
use cantrip_state::ChoiceRecord;

fn creatures(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, c: &Chars) -> bool {
    c.is(CardType::Creature)
}
fn four_four(_: &dyn LayerQuery, _: ObjectId, c: &mut Chars) {
    c.power = Some(4);
    c.toughness = Some(4);
}
static SETTER: CardDef = CardDef {
    name: "Probe base setter",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[static_ability(Layer::PtSet, creatures, four_four, "Creatures have base power and toughness 4/4.")],
    ..CardDef::BLANK
};

#[test]
fn characteristic_definition_precedes_an_older_base_setting_effect() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&SETTER]))));
    let filler = table.find("Plains").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, shuffle: false, ..Setup::default() });
    let mut add = |name, timestamp| {
        let mut obj = Obj::new(table.find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD);
        obj.timestamp = timestamp;
        let id = game.state.push_obj(obj);
        game.state.zone_push(Zone::BATTLEFIELD, id);
        id
    };
    add("Probe base setter", 10);
    let race = add("Nameless Race", 20);
    let text = game.cards.linked_to(&game.state, race).text;
    let choices = cantrip_state::Choices::entered(text, cantrip_state::Entered { number: Some(2), ..Default::default() });
    game.state.update_obj(race, |o| o.marks.choices = choices);
    game.state.see_statics();
    let chars = game.chars(race);
    assert_eq!((chars.power, chars.toughness), (Some(4), Some(4)));
}

#[test]
fn no_payment_yet_defines_zero_in_graveyard() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all())));
    let filler = table.find("Plains").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, shuffle: false, ..Setup::default() });
    let zone = Zone::graveyard(PlayerId(0));
    let race = game.state.push_obj(Obj::new(table.find("Nameless Race").unwrap(), PlayerId(0), zone));
    game.state.zone_push(zone, race);
    let chars = game.chars(race);
    assert_eq!((chars.power, chars.toughness), (Some(0), Some(0)));
}

static ABILITY_REMOVER: CardDef = CardDef {
    name: "Probe ability remover",
    types: TypeSet::of(&[CardType::Enchantment]),
    statics: &[loses_all_abilities(creatures, "Creatures lose all abilities.")],
    ..CardDef::BLANK
};

#[test]
fn losing_the_cda_leaves_zero_for_each_printed_star() {
    let table = Box::leak(Box::new(CardTable::new(cantrip_cards::all().into_iter().chain([&ABILITY_REMOVER]))));
    let filler = table.find("Plains").unwrap();
    let mut game = Game::new(table, &Setup { decks: vec![vec![filler; 20], vec![filler; 20]], seed: 1, shuffle: false, ..Setup::default() });
    for name in ["Probe ability remover", "Nameless Race"] {
        let id = game.state.push_obj(Obj::new(table.find(name).unwrap(), PlayerId(0), Zone::BATTLEFIELD));
        game.state.zone_push(Zone::BATTLEFIELD, id);
    }
    game.state.see_statics();
    let race = *game.state.battlefield().last().unwrap();
    let chars = game.chars(race);
    assert_eq!((chars.power, chars.toughness), (Some(0), Some(0)));
}
