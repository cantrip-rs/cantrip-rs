use cantrip_api::*;
use cantrip_mechanics::{fight, gift, targets, tokens};
use cantrip_oracle as oracle;

/// "Choose target creature you control and target creature you don't control. Put a +1/+1 counter on the creature you control
/// if the gift was promised. Then those creatures fight each other." The gift was given before this runs (CR 702.174j). Each
/// target is judged as the spell resolves (CR 608.2b): with the creature you control gone nothing is put on and nothing
/// fights; with only the other gone the counter is still put on and nothing fights (CR 701.14b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if item.gift.is_some() {
        return fight::plus_one_counters_on_first_target_then_it_fights_second::<1>(g, item);
    }
    fight::targets_fight(g, item, 0, 1);
    Ok(())
}

/// "Gift a tapped Fish" (CR 702.174f): the Fish is the card's own token.
pub static EXTRA: &[&CardDef] = &[&tokens::FISH];

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_YOU_DONT_CONTROL], effect),
    gift: Some(gift::TAPPED_FISH),
    ..oracle::LONGSTALK_BRAWL
};
