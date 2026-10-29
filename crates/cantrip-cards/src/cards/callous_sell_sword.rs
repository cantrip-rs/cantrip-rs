use cantrip_api::*;
use cantrip_mechanics::{history, targets};
use cantrip_oracle as oracle;

/// "This creature enters with a +1/+1 counter on it for each creature that died under your control this turn"
/// (CR 614.1c, 700.4): counted as it enters, from the event history; it is not yet on the battlefield, so it is not one.
fn enters_with_counters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let died = history::creatures_died_this_turn(g, Some(entering.controller));
    entering.counters[CounterKind::PlusOne as usize] += died.min(i16::MAX as usize) as i16;
    Ok(())
}

/// "Target creature you control deals damage equal to its power to any other target. Then sacrifice it." The damage is
/// dealt by the creature (CR 120.3), so its keywords (deathtouch, lifelink) count. If the creature is no longer a legal
/// target nothing happens (CR 608.2b); if only the other target is gone the creature deals no damage but is still
/// sacrificed (official ruling).
fn burn_together(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    if g.target_legal(item, 1) {
        let power = g.chars(creature).power.unwrap_or(0).max(0) as u16;
        g.damage(creature, item.target(1), power);
    }
    g.sacrifice(item.controller, &[creature]);
    Ok(())
}

/// The Adventure (CR 715).
pub static BURN_TOGETHER: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal the damage, then be sacrificed"),
            targets::ANY_TARGET_OTHER_THAN_THOSE_CHOSEN.labelled("Target to be dealt damage equal to that creature's power"),
        ],
        burn_together,
    ),
    ..oracle::BURN_TOGETHER
};

pub static CARD: CardDef = CardDef {
    enters: Some(enters_with_counters),
    adventure: Some(&BURN_TOGETHER),
    ..oracle::CALLOUS_SELL_SWORD
};

pub static EXTRA: &[&CardDef] = &[&BURN_TOGETHER];
