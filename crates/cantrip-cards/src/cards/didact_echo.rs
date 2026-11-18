use cantrip_api::*;
use cantrip_mechanics::statics::{have_flying, itself_with_permanent_cards_in_your_graveyard};
use cantrip_mechanics::{effects, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "Descend 4 - This creature has flying as long as there are four or more permanent cards in your graveyard" (CR 207.2c: descend is
/// an ability word; 604.1, 611.3a, 613.1f): its controller's graveyard, counted whenever characteristics are computed.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When this creature enters, draw a card.")],
    statics: &[static_ability(
        Layer::Ability,
        itself_with_permanent_cards_in_your_graveyard::<4>,
        have_flying,
        "Descend 4 \u{2014} This creature has flying as long as there are four or more permanent cards in your graveyard.",
    )],
    ..oracle::DIDACT_ECHO
};
