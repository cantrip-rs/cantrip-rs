use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::return_first_target_to_hand, "When Daniels enters, return target creature card from your graveyard to your hand.")
            .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD]),
        ability(
            cost("{2}").and(put_self_into_library(LibraryPlace::Bottom)),
            |g, item| { g.gain_life(item.controller, 2); Ok(()) },
            "{2}, Put this card from your graveyard on the bottom of your library: You gain 2 life.",
        ).from(ZoneKind::Graveyard),
    ],
    ..oracle::DANIELS_TEMPORAL_AGENT
};
