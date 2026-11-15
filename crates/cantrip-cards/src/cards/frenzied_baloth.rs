use cantrip_api::*;
use cantrip_mechanics::statics::{cant_be_countered, your_spells_of_types};
use cantrip_oracle as oracle;

/// "Creature spells you control": a creature spell on the stack controlled by the player who controls this permanent.
const CREATURE: u16 = TypeSet::of(&[CardType::Creature]).0;

/// "Combat damage can't be prevented" (CR 615.12): the combat damage of every source, whoever controls it.
fn combat_damage(_: &dyn Query, _: &Asker, _: ObjectId, _: Target, combat: bool) -> bool {
    combat
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, your_spells_of_types::<CREATURE>, cant_be_countered, "Creature spells you control can't be countered.").everywhere()],
    unpreventable_damage: &[combat_damage],
    ..oracle::FRENZIED_BALOTH
};
