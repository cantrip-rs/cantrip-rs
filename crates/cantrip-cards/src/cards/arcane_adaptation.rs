use cantrip_api::*;
use cantrip_mechanics::chosen_type::{add_chosen_type, enters_choosing_a_creature_type, your_creatures_spells_and_cards};
use cantrip_oracle as oracle;

/// "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you
/// control and creature cards you own that aren't on the battlefield." A static ability that works on objects in every zone
/// (CR 611.3a), a type-changing effect in layer 4 (CR 613.1d) that adds the type chosen as it entered (CR 614.12a, the
/// `ChooseSubtype` question) and keeps the others (CR 205.1b). It is applied before any replacement effect looks at a
/// creature of that type entering (the ruling: with Metallic Mimic naming the same type, every creature enters with a
/// counter). Its reach - what is a creature - can change as other layer-4 effects apply (Opalescence), and the dependency
/// system orders them (CR 613.8). No type chosen (it entered with its abilities lost, CR 614.12), nothing is added.
pub static CARD: CardDef = CardDef {
    enters: Some(enters_choosing_a_creature_type),
    statics: &[static_ability(
        Layer::Type,
        your_creatures_spells_and_cards,
        add_chosen_type,
        "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.",
    )
    .everywhere()],
    ..oracle::ARCANE_ADAPTATION
};
