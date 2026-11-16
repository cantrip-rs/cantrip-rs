use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::statics::{itself_with_threshold, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{2}{U/B}"), |g, item| effects::surveil(g, item, 1), "{2}{U/B}: Surveil 1.")],
    statics: &[static_ability(
        Layer::PtModify,
        itself_with_threshold,
        plus_pt::<3, 0>,
        "Threshold \u{2014} As long as there are seven or more cards in your graveyard, this creature gets +3/+0.",
    )],
    ..oracle::MIND_DRILL_ASSAILANT
};
