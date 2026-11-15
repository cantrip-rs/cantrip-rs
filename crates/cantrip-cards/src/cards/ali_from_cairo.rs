use cantrip_api::*;
use cantrip_mechanics::replacements;
use cantrip_oracle as oracle;

/// "Damage that would reduce your life total to less than 1 reduces it to 1 instead." (CR 120.3a, 120.4c, 614.1a): only
/// the life damage costs its controller is replaced, never damage itself (the rulings), and it applies while Ali is on
/// the battlefield, damage dealt together with Ali's own lethal damage included.
pub static CARD: CardDef = CardDef { replacements: &[replacements::DAMAGE_CANT_REDUCE_YOUR_LIFE_BELOW_1], ..oracle::ALI_FROM_CAIRO };
