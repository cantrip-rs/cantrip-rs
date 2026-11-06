use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature gains first strike until end of turn. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 611.2a).
fn target_creature_gains_first_strike(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                effects::gain_until_end_of_turn(g, item.object, object, Keyword::FirstStrike);
            }
        },
    );
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
    spell: spell(&[targets::CREATURE], target_creature_gains_first_strike),
    delayed: &[
        delayed_trigger(
            |g, item| effects::draw(g, item, 1),
            "Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    ..oracle::LIGHTNING_BLOW
};
