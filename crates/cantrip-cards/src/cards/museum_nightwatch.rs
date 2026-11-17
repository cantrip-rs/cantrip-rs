use cantrip_api::*;
use cantrip_mechanics::{disguise, tokens, triggers};
use cantrip_oracle as oracle;

/// "When this creature dies, create a 2/2 white and blue Detective creature token." (CR 700.4, 603.10a, 111.1) The trigger looks
/// back at the creature as it left the battlefield (CR 603.10a): face up it has the ability, but face down (cast for {3} or not
/// yet turned up, CR 708.2) it has none, so a face-down Nightwatch that dies makes no Detective. Disguise {1}{W} (CR 702.168a).
pub static CARD: CardDef = CardDef {
    ways: &[disguise::way(Cost::mana(mana("{1}{W}")), "Disguise {1}{W}")],
    abilities: &[triggered(triggers::THIS_DIES, tokens::create_a_white_blue_detective, "When this creature dies, create a 2/2 white and blue Detective creature token.")],
    ..oracle::MUSEUM_NIGHTWATCH
};
pub static EXTRA: &[&CardDef] = &[&tokens::WHITE_BLUE_DETECTIVE];
