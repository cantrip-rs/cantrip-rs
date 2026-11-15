use cantrip_api::*;
use cantrip_mechanics::{gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Destroy target creature. If the gift wasn't promised, you lose 2 life." The gift itself was given before this runs
/// (CR 702.174j). The only target is checked on resolution, so a spell whose target is illegal never resolves: no creature
/// is destroyed and no life is lost (CR 608.2b). The life is lost whether or not the creature was destroyed.
fn destroy_it_and_lose_life_unless_gifted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.destroy(creature);
    if item.gift.is_none() {
        g.lose_life(item.controller, 2);
    }
    Ok(())
}

/// "Gift a Food" (CR 702.174d): the Food is a token of its own.
pub static EXTRA: &[&CardDef] = &[&tokens::FOOD];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], destroy_it_and_lose_life_unless_gifted),
    gift: Some(gift::FOOD),
    ..oracle::NOCTURNAL_HUNGER
};
