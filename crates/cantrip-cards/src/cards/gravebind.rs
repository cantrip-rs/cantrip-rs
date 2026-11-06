use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature can't be regenerated this turn. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 611.2a).
fn target_creature_cant_be_regenerated(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::CantBeRegenerated,
                    Duration::EndOfTurn,
                );
            }
        },
    );
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::StepOfNextTurn {
            step: Step::Upkeep,
            of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], target_creature_cant_be_regenerated),
    delayed: &[
        delayed_trigger(|g, item| effects::draw(g, item, 1), "Draw a card at the beginning of the next turn's upkeep."),
    ],
    ..oracle::GRAVEBIND
};
