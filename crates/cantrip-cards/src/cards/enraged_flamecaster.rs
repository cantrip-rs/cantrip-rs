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
        when_asked(EventMask::SPELL_CAST, triggers::you_cast_a_spell_with_mana_value_at_least::<4>),
        flame,
        "Whenever you cast a spell with mana value 4 or greater, this creature deals 2 damage to each opponent.",
    )],
    ..oracle::ENRAGED_FLAMECASTER
};
