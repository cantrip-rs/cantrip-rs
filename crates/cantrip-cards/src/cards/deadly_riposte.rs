use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Deadly Riposte deals 3 damage to target tapped creature and you gain 2 life": the life comes with the damage and is lost
/// with it when the one target is gone or illegal as the spell resolves, since the spell then does not resolve at all (CR 608.2b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::TAPPED_CREATURE], |g, item| effects::damage_target_and_gain_life(g, item, 3, 2)),
    ..oracle::DEADLY_RIPOSTE
};
