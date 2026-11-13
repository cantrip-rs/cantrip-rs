use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, targets};
use cantrip_oracle as oracle;

/// "Enchant permanent" (CR 303.4a) and "You control enchanted permanent." (CR 613.1b, layer 2), at the Aura's timestamp (CR 613.7e),
/// for as long as it is attached.
pub static CARD: CardDef = CardDef {
    enchant: Some(targets::PERMANENT),
    statics: &[control_changing(attached_object, "You control enchanted permanent.")],
    ..oracle::CONFISCATE
};
