use cantrip_api::*;
use cantrip_mechanics::chosen_type::{add_chosen_type, enters_choosing_a_creature_type, your_creatures_spells_and_cards};
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "If this card is in your opening hand, you may begin the game with it on the battlefield" (CR 103.6), "As this enchantment
/// enters, choose a creature type" (CR 614.12a) and "Creatures you control are the chosen type in addition to their other types.
/// The same is true for creature spells you control and creature cards you own that aren't on the battlefield": a layer 4 static
/// ability that works in every zone (CR 611.3a, 613.1d), as Arcane Adaptation's, whose text it shares. Beginning the game with it
/// still asks for the type as it enters.
pub static CARD: CardDef = CardDef {
    opening_hand: Some(statics::BEGIN_ON_THE_BATTLEFIELD),
    enters: Some(enters_choosing_a_creature_type),
    statics: &[static_ability(
        Layer::Type,
        your_creatures_spells_and_cards,
        add_chosen_type,
        "Creatures you control are the chosen type in addition to their other types. The same is true for creature spells you control and creature cards you own that aren't on the battlefield.",
    )
    .everywhere()],
    ..oracle::LEYLINE_OF_TRANSFORMATION
};
