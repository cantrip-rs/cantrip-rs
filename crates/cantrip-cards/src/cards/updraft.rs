use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Grant flying, then schedule a draw for the next turn's upkeep (CR 611.2a, 603.7b).
fn updraft(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::first_target_gains_flying(g, item)?;
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
    spell: spell(&[targets::CREATURE], updraft),
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::UPDRAFT
};
