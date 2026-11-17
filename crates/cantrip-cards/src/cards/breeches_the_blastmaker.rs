use cantrip_api::*;
use cantrip_mechanics::{copy, effects, flurry, select, targets};
use cantrip_oracle as oracle;

/// "You may sacrifice an artifact. If you do, flip a coin. When you win the flip, ... When you lose the flip, ..." (CR 701.21a,
/// 705.1, 603.12): the artifact is chosen as the ability resolves; with none sacrificed no coin is flipped. The flip's result
/// makes one of two reflexive abilities, each about the spell (its last-known information if it has left the stack, CR 608.2h);
/// the losing one's target is chosen as it goes on the stack.
fn sacrifice_and_flip(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::may_sacrifice(g, item.controller, item.object, select::artifact_cards(select::up_to(1)))?.is_empty() {
        return Ok(());
    }
    let won = g.flip_coin(item.controller);
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, if won { 0 } else { 1 }), item.subject);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[flurry::trigger(
        "Whenever you cast your second spell each turn, you may sacrifice an artifact. If you do, flip a coin. When you win the flip, copy that spell. You may choose new targets for the copy. When you lose the flip, Breeches deals damage equal to that spell's mana value to any target.",
        sacrifice_and_flip,
    )],
    delayed: &[
        delayed_trigger(copy::copy_the_cast_spell, "When you win the flip, copy that spell. You may choose new targets for the copy."),
        delayed_trigger(effects::damage_first_target_equal_to_the_spells_mana_value, "When you lose the flip, Breeches deals damage equal to that spell's mana value to any target.")
            .targets(&[targets::ANY]),
    ],
    ..oracle::BREECHES_THE_BLASTMAKER
};
