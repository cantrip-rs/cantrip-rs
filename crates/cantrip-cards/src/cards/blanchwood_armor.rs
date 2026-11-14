use cantrip_api::*;
use cantrip_mechanics::{attach, lands, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature gets +1/+1 for each Forest you control." (CR 613.4c, layer 7c, 303.4): "you" is the Aura's controller
/// (CR 109.5), wherever the enchanted creature is, so the enchanted creature of another player still counts the Armor's controller's Forests.
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::PtModify,
        attach::attached_object,
        lands::plus_per_subtype_you_control::<{ lands::sub(Subtype::Forest) }, false, 1, 1>,
        "Enchanted creature gets +1/+1 for each Forest you control.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::BLANCHWOOD_ARMOR
};
