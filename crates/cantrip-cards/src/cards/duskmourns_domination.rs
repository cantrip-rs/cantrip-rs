use cantrip_api::*;
use cantrip_mechanics::{attach, statics, targets};
use cantrip_oracle as oracle;

/// "Enchant creature" (CR 303.4a), "You control enchanted creature." (CR 613.1b, layer 2) and "Enchanted creature gets -3/-0
/// and loses all abilities." (CR 613.1f, layer 6; 613.4c, layer 7c): all at the Aura's timestamp (CR 613.7e), so an ability the
/// creature gains later is kept (ruling 2024-09-20). Control Magic's control change with Fresh Start's -N/-0 and loss of
/// abilities.
pub static CARD: CardDef = CardDef {
    statics: &[
        control_changing(attach::attached_object, "You control enchanted creature."),
        loses_all_abilities(attach::attached_object, "Enchanted creature gets -3/-0 and loses all abilities."),
        static_ability(
            Layer::PtModify,
            attach::attached_object,
            statics::plus_pt::<-3, 0>,
            "Enchanted creature gets -3/-0 and loses all abilities.",
        )
        .continues(),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::DUSKMOURNS_DOMINATION
};
