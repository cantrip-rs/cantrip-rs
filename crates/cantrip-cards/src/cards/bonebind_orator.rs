use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{3}{B}").and(EXILE_THIS),
        effects::return_first_target_to_hand,
        "{3}{B}, Exile this card from your graveyard: Return another target creature card from your graveyard to your hand.",
    )
    .targets(&[targets::ANOTHER_CREATURE_CARD_IN_YOUR_GRAVEYARD])
    .from(ZoneKind::Graveyard)],
    ..oracle::BONEBIND_ORATOR
};
