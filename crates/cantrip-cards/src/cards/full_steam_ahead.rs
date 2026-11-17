use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Until end of turn, each creature you control gets +2/+2 and gains trample and 'This creature can't be blocked by more than
/// one creature.'" (CR 611.2c, 702.19, 509.1b): the creatures its controller controls as it resolves, none that enter later. The
/// granted ability is the core keyword `CantBeBlockedByMoreThanOne`, which is what that text is.
pub static CARD: CardDef = CardDef {
    spell: spell(
        &[],
        effects::creatures_you_control_get_and_gain::<2, 2, { Keywords::of(&[Keyword::Trample, Keyword::CantBeBlockedByMoreThanOne]).0 }>,
    ),
    ..oracle::FULL_STEAM_AHEAD
};
