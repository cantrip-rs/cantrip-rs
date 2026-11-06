use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Cast this spell only during combat before blockers are declared." (CR 601.3).
fn cast_spell_only_during_combat_if(q: &dyn Query, _: &Asker) -> bool {
    ((Step::BeginCombat as u8)..=(Step::EndOfCombat as u8))
        .contains(&(q.state().turn.step() as u8))
        && (q.state().turn.step() as u8) < (Step::DeclareBlockers as u8)
}

/// "Cast this spell only during combat before blockers are declared. Target creature can't block this turn. Draw a card at the beginning of the next turn's upkeep." (CR 608.2, 611.2a).
fn cast_spell_only_during_combat(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    effects::first_target_cant_block_this_turn(g, item)?;
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
    spell: spell(&[targets::CREATURE], cast_spell_only_during_combat),
    delayed: &[
        delayed_trigger(
            |g, item| effects::draw(g, item, 1),
            "Draw a card at the beginning of the next turn's upkeep.",
        ),
    ],
    cast_only_if: Some(cast_spell_only_during_combat_if),
    ..oracle::PANIC
};
