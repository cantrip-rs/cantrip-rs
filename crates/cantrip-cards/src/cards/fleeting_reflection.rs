use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gains hexproof until end of turn. Untap that creature. Until end of turn, it becomes a copy of up
/// to one other target creature." Each target is checked as the spell resolves (CR 608.2b): with the first gone the spell does
/// nothing to it, and with the second gone or not chosen it still gains hexproof and untaps but copies nothing. The copy is of the
/// copiable values (CR 707.2) of the other creature as they are then, so a creature that is itself a copy gives its copy's values,
/// and it is still the same object: it stays on the battlefield without entering or leaving (CR 707.2).
fn gain_hexproof_untap_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_hexproof_until_end_of_turn(g, item.object, creature);
    g.end_sentence();
    g.untap(creature);
    g.end_sentence();
    let Some(copied) = targets::object(g, item, 1) else { return Ok(()) };
    let values = g.copiable_values(copied);
    g.become_copy(item.object, creature, values, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_OTHER_THAN_THOSE_CHOSEN.optional()], gain_hexproof_untap_and_copy),
    ..oracle::FLEETING_REFLECTION
};
