use cantrip_api::*;
use cantrip_mechanics::statics::{itself_with_permanent_cards_in_your_graveyard, plus_pt};
use cantrip_oracle as oracle;

/// "Descend 4 - This creature gets +3/+0 as long as there are four or more permanent cards in your graveyard" (CR 207.2c: descend is
/// an ability word; 604.1, 611.3a, 613.4c): its controller's graveyard, counted whenever characteristics are computed.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_permanent_cards_in_your_graveyard::<4>,
        plus_pt::<3, 0>,
        "Descend 4 \u{2014} This creature gets +3/+0 as long as there are four or more permanent cards in your graveyard.",
    )],
    ..oracle::BASKING_CAPYBARA
};
