use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Creatures you control get +10/+10 and gain vigilance until end of turn." (CR 611.2, 611.2c): the creatures there are as it
/// resolves, and no creature that enters later is affected.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_get_and_gain::<10, 10, { Keywords::of(&[Keyword::Vigilance]).0 }>),
    ..oracle::PREPOSTEROUS_PROPORTIONS
};
