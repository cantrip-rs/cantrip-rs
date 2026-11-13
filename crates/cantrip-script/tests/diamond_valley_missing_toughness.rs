//! CR 208.5 and 107.2: a creature without a toughness value has toughness zero.
//! A test static sets toughness to one until its source is sacrificed for a mana tax while costs are being paid.
use std::sync::OnceLock;
use cantrip_engine::*;
use cantrip_script::{run_on, GameScript};

static CREATURE: CardDef = CardDef {
    name: "Probe Creature Without Toughness",
    oracle: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(8),
    toughness: None,
    ..CardDef::BLANK
};

fn other_creatures(_: &dyn LayerQuery, source: ObjectId, affected: ObjectId, chars: &Chars) -> bool {
    source != affected && chars.is(CardType::Creature)
}
fn set_toughness(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.toughness = Some(1);
}
static LIFELINE: CardDef = CardDef {
    name: "Probe Toughness Setter",
    oracle: true,
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(1), toughness: Some(1),
    statics: &[static_ability(Layer::PtSet, other_creatures, set_toughness, "Other creatures have base toughness 1.")],
    ..CardDef::BLANK
};

fn table() -> &'static CardTable {
    static TABLE: OnceLock<CardTable> = OnceLock::new();
    TABLE.get_or_init(|| CardTable::new(cantrip_cards::all().into_iter().chain([&CREATURE, &LIFELINE])))
}

#[test]
fn missing_toughness_is_zero_when_sacrificed() {
    let original = include_str!("../../../scenarios/diamond-valley-zero-toughness-during-payment.cantrip");
    let actions = original.split("[Expect]").next().unwrap();
    let script_text = actions.replace("Force of Savagery", "Probe Creature Without Toughness")
        .replace("Captain Jean-Luc Picard", "Probe Toughness Setter")
        .replace("? Bot1.Life = 20\n", "");
    let script = GameScript::parse(&script_text, "diamond-valley-no-toughness").unwrap();
    let result = run_on(&script, table()).unwrap_or_else(|failure| panic!("{}", failure.report));
    assert_eq!(result.game.state.player(PlayerId(0)).life, 20);
}
