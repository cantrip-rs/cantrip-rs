use cantrip_api::*;
use cantrip_mechanics::{pump, targets, tokens};
use cantrip_oracle as oracle;

/// "Up to one target creature gets -1/-1 until end of turn. You create a 1/1 black Rat creature token with 'This token can't
/// block.'" (CR 115.1a, 608.2b, 701.7): the shrink is done only if the one target chosen is still legal; the Rat is made anyway,
/// even when no target was chosen. Only a spell whose targets were all chosen and are all illegal is countered on resolution, and
/// a spell with none chosen is not. The sentence ends before the token is made (CR 608.2c): a creature the shrink kills is dead as
/// state-based actions are checked, after the whole spell.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        pump(g, item.object, creature, -1, -1);
    }
    g.end_sentence();
    tokens::create_rats_that_cant_block::<1>(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE.optional()], effect), ..oracle::RAT_OUT };

pub static EXTRA: &[&CardDef] = &[&tokens::RAT_CANT_BLOCK];
