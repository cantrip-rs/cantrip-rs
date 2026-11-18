use cantrip_api::*;
use cantrip_mechanics::evolve::{greater_creature_you_control_enters, is_greater};
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Put a +1/+1 counter on this creature and investigate" (CR 122.1, 701.16a), if the intervening "if" still holds as it resolves
/// (CR 603.4): the creature that entered is still greater in power or toughness, as it is now or as it last was (the same comparison
/// evolve makes), or else the ability does nothing - no counter and no Clue.
fn counter_and_investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(entered) = item.subject else { return Ok(()) };
    if is_greater(g, item.object, entered) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
        tokens::investigate(g, item.controller)?;
    }
    Ok(())
}

/// Vigilance is a core keyword. The trigger is evolve's (CR 702.100a) in words: any creature you control entering, this one
/// included, that has greater power or toughness than this creature.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, greater_creature_you_control_enters),
        counter_and_investigate,
        "Whenever a creature you control enters, if its power is greater than this creature's power or its toughness is greater than this creature's toughness, put a +1/+1 counter on this creature and investigate.",
    )],
    ..oracle::SHARP_EYED_ROOKIE
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
