use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, select};
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, you may sacrifice another creature. If you do, draw a card and this creature can't be
/// blocked this turn." The sacrifice is chosen as the ability resolves and may be declined (CR 608.2d, 701.21a); the card and
/// the evasion come only if a creature was actually sacrificed (CR 608.2c).
fn sacrifice_to_draw_and_evade(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let rule = select::another_creature_permanents(select::up_to(1));
    if !effects::may_sacrifice(g, item.controller, item.object, rule)?.is_empty() {
        effects::draw(g, item, 1)?;
        effects::this_cant_be_blocked_this_turn(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        sacrifice_to_draw_and_evade,
        "Whenever this creature attacks, you may sacrifice another creature. If you do, draw a card and this creature can't be blocked this turn.",
    )],
    ..oracle::VAMPIRE_GOURMAND
};
