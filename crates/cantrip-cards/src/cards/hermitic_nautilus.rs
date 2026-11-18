use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

// Vigilance is the core's, read from the card's JSON.
pub static CARD: CardDef = CardDef {
    abilities: &[ability(cost("{1}{U}"), effects::this_gets::<3, -3>, "{1}{U}: This creature gets +3/-3 until end of turn.")],
    ..oracle::HERMITIC_NAUTILUS
};
