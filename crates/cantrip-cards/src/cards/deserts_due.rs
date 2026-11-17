use cantrip_api::*;
use cantrip_mechanics::{pump, targets};
use cantrip_oracle as oracle;

/// "Target creature gets -2/-2 until end of turn. It gets an additional -1/-1 until end of turn for each Desert you control."
/// (CR 611.2a, 608.2b): the Deserts are counted as the spell resolves, and nothing happens if the target is no longer legal. The
/// Desert is the subtype read through the words of this card's text (CR 612.2); the count is one effect (CR 611.2a), so a creature
/// with toughness 3 and one Desert is -3/-3 as a whole.
fn target_creature_gets_minus_two_and_one_more_for_each_desert(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let desert = g.words(item.object).subtype(Subtype::Desert);
    let deserts = cantrip_mechanics::permanents_with_subtype_you_control(g, item.controller, desert) as i32;
    pump(g, item.object, creature, -2 - deserts, -2 - deserts);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], target_creature_gets_minus_two_and_one_more_for_each_desert),
    ..oracle::DESERTS_DUE
};
