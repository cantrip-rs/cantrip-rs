use cantrip_api::*;
use cantrip_mechanics::{attach, effects, statics, targets};
use cantrip_oracle as oracle;

pub static CARIBOU: CardDef = CardDef {
    types: TypeSet::of(&[CardType::Creature]),
    colors: Some(ColorSet(1)),
    power: Some(0),
    toughness: Some(1),
    ..token!(Caribou)
};

/// "{W}{W}, {T}: Create a 0/1 white Caribou creature token." (CR 608.2).
fn granted_0(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut tokens_1 = Vec::new();
    let def_1_0 = g.card_id(&CARIBOU);
    tokens_1.extend(std::iter::repeat((def_1_0, item.controller)).take(1));
    let words_1 = g.words(item.object);
    g.create_tokens_worded(&tokens_1, false, None, words_1)?;
    Ok(())
}

/// "Sacrifice a Caribou token: You gain 1 life." (CR 608.2).
fn gain_1_life_2(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Caribou))
        && q.state().obj(id).token
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
            sacrifice(1, gain_1_life_2),
            effects::gain_one_life,
            "Sacrifice a Caribou token: You gain 1 life.",
        ),
    ],
    statics: &[
        static_ability(
            Layer::Ability,
            attach::attached_object,
            statics::grant_delayed_ability,
            "Enchanted land has \"{W}{W}, {T}: Create a 0/1 white Caribou creature token.\"",
        ),
    ],
    delayed: &[
        ability(
            cost("{W}{W}").and(TAP),
            granted_0,
            "{W}{W}, {T}: Create a 0/1 white Caribou creature token.",
        ),
    ],
    enchant: Some(targets::LAND_YOU_CONTROL),
    ..oracle::CARIBOU_RANGE
};

pub static EXTRA: &[&CardDef] = &[&CARIBOU];
