use cantrip_api::*;
use cantrip_mechanics::{cumulative_upkeep, mana, targets};
use cantrip_mechanics::cumulative_upkeep::Payment;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[cumulative_upkeep!(
        Payment::Sacrifice(targets::filters::land),
        "Cumulative upkeep—Sacrifice a land. (At the beginning of your upkeep, put an age counter on this permanent, then sacrifice it unless you pay its upkeep cost for each age counter on it.)"
    )],
    enters: Some(mana::enters_tapped),
    ..oracle::POLAR_KRAKEN
};
