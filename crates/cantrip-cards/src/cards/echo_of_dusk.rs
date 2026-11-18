use cantrip_api::*;
use cantrip_mechanics::statics::{have_lifelink, itself_with_permanent_cards_in_your_graveyard, plus_pt};
use cantrip_oracle as oracle;

/// "Descend 4 - As long as there are four or more permanent cards in your graveyard, this creature gets +1/+1 and has lifelink"
/// (CR 207.2c, 700.11, 604.1, 613.4c): two static effects, the keyword in layer 6 and +1/+1 in layer 7c, each asked again
/// whenever characteristics are computed, counting the permanent cards in its controller's graveyard.
pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::Ability,
            itself_with_permanent_cards_in_your_graveyard::<4>,
            have_lifelink,
            "Descend 4 \u{2014} As long as there are four or more permanent cards in your graveyard, this creature gets +1/+1 and has lifelink.",
        ),
        static_ability(
            Layer::PtModify,
            itself_with_permanent_cards_in_your_graveyard::<4>,
            plus_pt::<1, 1>,
            "Descend 4 \u{2014} As long as there are four or more permanent cards in your graveyard, this creature gets +1/+1 and has lifelink.",
        ),
    ],
    ..oracle::ECHO_OF_DUSK
};
