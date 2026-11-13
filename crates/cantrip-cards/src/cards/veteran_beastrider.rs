use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your end step, untap each creature you control." (CR 513.1, 701.26b): the creatures its controller
/// controls as it resolves, untapped together, whichever player's turn it was not. "{2}{G}{W}: Creatures you control get +1/+1 until
/// end of turn." (CR 611.2c): the creatures it controls as it resolves, and none that enters later.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_END_STEP,
            effects::untap_all_creatures_you_control,
            "At the beginning of your end step, untap each creature you control.",
        ),
        ability(
            cost("{2}{G}{W}"),
            effects::creatures_you_control_get::<1, 1>,
            "{2}{G}{W}: Creatures you control get +1/+1 until end of turn.",
        ),
    ],
    ..oracle::VETERAN_BEASTRIDER
};
