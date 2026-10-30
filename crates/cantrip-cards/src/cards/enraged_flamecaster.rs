use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "This creature deals 2 damage to each opponent" (CR 120.3).
fn flame(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOU_CAST_A_SPELL_WITH_MANA_VALUE_4_OR_GREATER,
        flame,
        "Whenever you cast a spell with mana value 4 or greater, this creature deals 2 damage to each opponent.",
    )],
    ..oracle::ENRAGED_FLAMECASTER
};
