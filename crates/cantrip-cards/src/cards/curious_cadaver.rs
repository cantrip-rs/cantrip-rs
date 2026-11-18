use cantrip_api::*;
use cantrip_mechanics::effects::return_this_to_hand;
use cantrip_mechanics::{lands, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        // It returns this card from the graveyard, so it functions only there (CR 113.6m).
        triggered(
            when_asked(EventMask::ZONE_CHANGE, triggers::you_sacrifice_a_subtype::<{ lands::sub(Subtype::Clue) }>),
            return_this_to_hand,
            "When you sacrifice a Clue, return this card from your graveyard to your hand.",
        )
        .on_sacrifice()
        .functions_in(IN_A_GRAVEYARD),
    ],
    ..oracle::CURIOUS_CADAVER
};
