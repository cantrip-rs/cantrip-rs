use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature gets +2/+2, gains flying, and becomes a Horror enchantment creature in addition to its
/// other types" (CR 611.2a, 205.1b, 613.1d): one instruction from the spell, on the target if it is still legal as the spell
/// resolves (CR 608.2b). The type and subtype are layer 4 effects, the Horror read through the words of the spell's text (CR 612).
fn pump_fly_and_become_a_horror_enchantment(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| {
        cantrip_mechanics::pump(g, item.object, creature, 2, 2);
        effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
        g.add_effect(item.object, creature, EffectKind::AddTypes(TypeSet::of(&[CardType::Enchantment, CardType::Creature])), Duration::EndOfTurn);
        effects::becomes_subtype_until_end_of_turn(g, item.object, creature, Subtype::Horror);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], pump_fly_and_become_a_horror_enchantment),
    ..oracle::JUMP_SCARE
};
