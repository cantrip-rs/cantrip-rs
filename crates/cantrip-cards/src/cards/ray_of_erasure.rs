use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Target player mills one card, then a draw is scheduled for the next turn’s upkeep (CR 701.17a, 603.7).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_player_mills::<1>(g, item)?;
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn { step: Step::Upkeep, of: None },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::PLAYER], effect),
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::RAY_OF_ERASURE
};
