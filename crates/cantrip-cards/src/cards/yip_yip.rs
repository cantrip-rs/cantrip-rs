use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +2/+2 until end of turn. If that creature is an Ally, it also gains flying until end of
/// turn." (CR 611.2a, 608.2b): the pump, then the Ally check (read through the words of this card's text, CR 612), made as the
/// spell resolves, on the creature as it is then; nothing if the target is no longer legal.
fn pump_and_fly_if_ally(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let ally = g.words(item.object).subtype(Subtype::Ally);
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    pump(g, item.object, creature, 2, 2);
    g.end_sentence();
    if g.chars(creature).has_subtype(ally) {
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE_YOU_CONTROL], pump_and_fly_if_ally), ..oracle::YIP_YIP };
