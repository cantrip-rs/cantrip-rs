use cantrip_api::*;
use cantrip_mechanics::{effects, pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Target player investigates. Target creature gets +1/+0 and gains flying until end of turn. Untap it." (CR 701.16a, 611.2a,
/// 701.26b): the player creates a Clue under their control, if they are still a legal target (CR 608.2b); each sentence is
/// judged on its own target, so a creature that became illegal leaves the investigation intact and the reverse. The pump and
/// the flying are one instruction on the creature, the untap the next sentence.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        tokens::investigate(g, player)?;
    }
    g.end_sentence();
    if let Some(creature) = targets::object(g, item, 1) {
        instruction(g, |g| {
            pump(g, item.object, creature, 1, 0);
            effects::gain_until_end_of_turn(g, item.object, creature, Keyword::Flying);
        });
        g.end_sentence();
        g.untap(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::PLAYER.labelled("Player who investigates"), targets::CREATURE.labelled("Creature to get +1/+0 and flying, then untap")],
        effect,
    ),
    ..oracle::PANTHER_POUNCE
};
