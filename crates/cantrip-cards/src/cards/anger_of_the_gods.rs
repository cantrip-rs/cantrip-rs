use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_mechanics::replacements::{CREATURE_EXILED_INSTEAD_OF_DYING, exile_if_it_would_die};
use cantrip_oracle as oracle;

/// "Anger of the Gods deals 3 damage to each creature. If a creature dealt damage this way would die this turn, exile it
/// instead." One damage event for every creature on the battlefield as the effect begins (CR 120.2b, 608.2c). The
/// replacement is the spell's own effect (CR 611.2c, 614.1a) on each permanent actually dealt damage by it (CR 120.3): none
/// for damage that was prevented, including a permanent a redirection sent it to (CR 614.9). The recipient must be a creature
/// when it would die, even if it was a planeswalker when damaged. It covers any way the creature would
/// die this turn, not only lethal damage (CR 700.4).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let before = g.history().head() as usize;
    effects::damage_each_creature(g, item, 3);
    let dealt = history::permanents_dealt_damage_by(g.history(), item.object, before);
    g.end_sentence();
    instruction(g, |g| {
        for creature in dealt {
            exile_if_it_would_die(g, item, 0, creature);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], effect),
    granted_replacements: &[CREATURE_EXILED_INSTEAD_OF_DYING.text("If a creature dealt damage this way would die this turn, exile it instead.")],
    ..oracle::ANGER_OF_THE_GODS
};
