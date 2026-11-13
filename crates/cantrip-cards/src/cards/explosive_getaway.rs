use cantrip_api::*;
use cantrip_mechanics::{delayed, effects, targets};
use cantrip_oracle as oracle;

/// "Exile up to one target artifact or creature. Return it to the battlefield under its owner's control at the beginning of the next
/// end step. [This spell] deals 4 damage to each creature." (CR 608.2c): the exile and the delayed return come first, so the exiled
/// creature is not on the battlefield to be dealt damage, and a creature that returns at the end step is a new object (CR 400.7).
fn exile_then_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::exile_target_then_return_at_next_end_step_0(g, item)?;
    effects::damage_each_creature(g, item, 4);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[up_to_one(targets::artifact_or_creature_filter)], exile_then_damage),
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::EXPLOSIVE_GETAWAY
};
