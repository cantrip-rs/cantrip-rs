use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature has double strike" (CR 702.4, 303.4e): a layer 6 grant (CR 613.1f) to the creature the Aura is attached to,
/// whoever controls it. "Enchant creature" is the Aura's target as it is cast (CR 303.4a); Flash is the oracle's keyword.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, attach::attached_object, statics::have_double_strike, "Enchanted creature has double strike.")],
    enchant: Some(targets::CREATURE),
    ..oracle::TWINBLADE_BLESSING
};
