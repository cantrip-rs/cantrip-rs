use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "You gain 1 life. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 119.3).
fn gain_1_life_draw_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.gain_life(item.controller, 1);
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
    spell: spell(&[], gain_1_life_draw_card),
    delayed: &[
        delayed_trigger(
            |g, item| effects::draw(g, item, 1),
            "Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    ..oracle::BLESSED_WINE
};
