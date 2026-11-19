use cantrip_api::*;
use cantrip_mechanics::{history, statics};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    // "Celebration \u{2014} This creature gets +1/+0 and has lifelink as long as two or more nonland permanents entered the
    // battlefield under your control this turn." Two static abilities that ask the same history question
    // (`history::this_while_you_celebrate`): the layers apply them again whenever characteristics are computed.
    statics: &[
        static_ability(
            Layer::PtModify,
            history::this_while_you_celebrate,
            statics::plus_pt::<1, 0>,
            "Celebration \u{2014} This creature gets +1/+0 as long as two or more nonland permanents entered the battlefield under your control this turn.",
        ),
        static_ability(
            Layer::Ability,
            history::this_while_you_celebrate,
            statics::have_lifelink,
            "Celebration \u{2014} This creature has lifelink as long as two or more nonland permanents entered the battlefield under your control this turn.",
        ),
    ],
    ..oracle::TUINVALE_GUIDE
};
