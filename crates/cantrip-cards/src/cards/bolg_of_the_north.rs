use cantrip_api::*;
use cantrip_mechanics::amass::amass;
use cantrip_mechanics::{effects, select, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "When Bolg enters, you may sacrifice another creature. When you do, ..." (CR 701.21, 603.12): the sacrifice is a choice made as
/// the ability resolves, not a cost, so a player with no other creature is not asked, and declining is the empty answer. The
/// reflexive ability triggers only when a creature was sacrificed, and carries that creature's power as it last existed on the
/// battlefield (the ruling): the old object the sacrifice's zone change in the history names, its pumps and counters included; a
/// negative power is no damage.
fn may_sacrifice_another_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::another_creature_permanents(select::up_to(1));
    let Some(&card) = effects::may_sacrifice(g, you, item.object, rule)?.first() else { return Ok(()) };
    let power = g
        .history()
        .all()
        .iter()
        .rev()
        .find_map(|r| match r.event {
            Event::ZoneChange { old, new, reason: ZoneReason::Sacrifice, .. } if new == card => Some(old),
            _ => None,
        })
        .map_or(0, |creature| g.chars(creature).power.unwrap_or(0).max(0) as u16);
    let this = g.card_id(&CARD);
    g.reflexive_trigger_counted(item.object, you, AbilityRef::delayed(this, 0), Target::None, power);
    Ok(())
}

/// "When you do, Bolg deals damage equal to that creature's power to another target creature. If excess damage was dealt this way,
/// amass Goblins X, where X is that excess damage." The damage is dealt by Bolg as he last existed if he has left (CR 608.2h), and
/// the excess is what the damage procedure found after replacement and prevention (CR 120.10), on the creature it really went to.
fn deal_damage_and_amass_the_excess(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    let target = Target::Object(creature);
    if let Some(result) = g.damage_result(item.object, target, item.x as u16)
        && result.target == target
        && result.excess > 0
    {
        g.end_sentence();
        // A counter count holds at most what the engine's width does (RX-5).
        amass(g, item.controller, item.object, &tokens::GOBLIN_ARMY, Subtype::Goblin, result.excess.min(i16::MAX as u32) as i16)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        may_sacrifice_another_creature,
        "When Bolg enters, you may sacrifice another creature. When you do, Bolg deals damage equal to that creature's power to another target creature. If excess damage was dealt this way, amass Goblins X, where X is that excess damage.",
    )],
    delayed: &[delayed_trigger(
        deal_damage_and_amass_the_excess,
        "When you do, Bolg deals damage equal to that creature's power to another target creature. If excess damage was dealt this way, amass Goblins X, where X is that excess damage.",
    )
    .targets(&[targets::OTHER_CREATURE])],
    ..oracle::BOLG_OF_THE_NORTH
};

pub static EXTRA: &[&CardDef] = &[&tokens::GOBLIN_ARMY];
