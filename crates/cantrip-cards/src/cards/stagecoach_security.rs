use cantrip_api::*;
use cantrip_mechanics::{effects, plot, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::creatures_you_control_get_and_gain::<1, 1, { Keywords::of(&[Keyword::Vigilance]).0 }>,
            "When this creature enters, creatures you control get +1/+1 and gain vigilance until end of turn.",
        ),
        plot::plot(mana("{3}{W}"), "Plot {3}{W}"),
    ],
    ..oracle::STAGECOACH_SECURITY
};
