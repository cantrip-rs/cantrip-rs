use cantrip_api::*;
use cantrip_mechanics::{library, targets};
use cantrip_mechanics::triggers::YOUR_UPKEEP;
use cantrip_oracle as oracle;

/// Omit Variables, the prepare spell (CR 722): "Mill three cards."
pub static OMIT_VARIABLES: CardDef = CardDef {
    name: "Omit Variables",
    cost: mana("{U/B}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[], |g, item| {
        g.mill(item.controller, 3);
        Ok(())
    }),
    text: "Mill three cards.",
    ..CardDef::BLANK
};

/// The intervening "if this creature isn't prepared" (CR 603.4), checked as the ability triggers and as it resolves.
fn this_is_not_prepared(q: &dyn Query, me: &Asker) -> bool {
    !q.obj(me.source).prepared
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            YOUR_UPKEEP,
            cantrip_mechanics::prepare::become_prepared,
            "At the beginning of your upkeep, if this creature isn't prepared, it becomes prepared. (While it's prepared, you may cast a copy of its spell. Doing so unprepares it.)",
        )
        .condition(this_is_not_prepared),
        ability(cost("{2}"), library::put_first_target_on_the_bottom, "{2}: Put target card from your graveyard on the bottom of your library.")
            .targets(&[targets::CARD_IN_YOUR_GRAVEYARD]),
    ],
    prepare: Some(&OMIT_VARIABLES),
    ..oracle::PARADOX_SHAPER
};

pub static EXTRA: &[&CardDef] = &[&OMIT_VARIABLES];
