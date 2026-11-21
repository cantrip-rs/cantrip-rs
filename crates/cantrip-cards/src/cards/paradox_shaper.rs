use cantrip_api::*;
use cantrip_mechanics::{library, targets};
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

pub static CARD: CardDef = CardDef {
    abilities: &[
        cantrip_mechanics::prepare::AT_YOUR_UPKEEP_IF_NOT_PREPARED_BECOMES_PREPARED,
        ability(cost("{2}"), library::put_first_target_on_the_bottom, "{2}: Put target card from your graveyard on the bottom of your library.")
            .targets(&[targets::CARD_IN_YOUR_GRAVEYARD]),
    ],
    prepare: Some(&OMIT_VARIABLES),
    ..oracle::PARADOX_SHAPER
};

pub static EXTRA: &[&CardDef] = &[&OMIT_VARIABLES];
