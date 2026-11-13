use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effects::creatures_you_control_get_and_gain::<1, 0, { Keywords::of(&[Keyword::Indestructible]).0 }>),
    ..oracle::MAKE_A_STAND
};
