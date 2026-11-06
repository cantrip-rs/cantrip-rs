use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, draw a card at the beginning of the next turn's upkeep." (CR 603.7).
fn draw_card_beginning_next_turns(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
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
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        draw_card_beginning_next_turns,
        "When this creature enters, draw a card at the beginning of the next turn's upkeep.",
    )],
    delayed: &[delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep.")],
    ..oracle::PYKNITE
};
