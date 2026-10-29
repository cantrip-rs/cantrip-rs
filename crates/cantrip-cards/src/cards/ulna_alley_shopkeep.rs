use cantrip_api::*;
use cantrip_mechanics::statics::{itself_while_you_gained_life, plus_pt};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        itself_while_you_gained_life,
        plus_pt::<2, 0>,
        "Infusion \u{2014} This creature gets +2/+0 as long as you gained life this turn.",
    )],
    ..oracle::ULNA_ALLEY_SHOPKEEP
};
