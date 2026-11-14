use cantrip_api::*;
use cantrip_mechanics::{fight, pump, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature you don't control with flying" (CR 115.1, 109.5, 702.9), read from the layers as the ability goes on the
/// stack and again as it resolves (CR 608.2b).
fn creature_you_dont_control_with_flying(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE_YOU_DONT_CONTROL.filter)(q, asker, t) && matches!(t, Target::Object(id) if targets::has_flying_creature(q, id))
}

/// "Undergrowth \u{2014} When this creature enters, choose up to one target creature you don't control with flying. This creature
/// gets +X/+0 until end of turn, where X is the number of creature cards in your graveyard, then you may have this creature
/// fight that creature." X is counted once, as the ability resolves, in its controller's graveyard only (CR 608.2h, 109.5);
/// tokens are not cards. The pump happens with or without a target; with its one target illegal the ability does not resolve
/// (CR 608.2b). The fight is chosen at resolution, after the pump (CR 608.2c), and if Harpooner has left the battlefield
/// neither creature fights (CR 701.14b).
fn undergrowth(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = cantrip_mechanics::creature_cards_in_graveyard(g, item.controller) as i32;
    pump(g, item.object, item.object, x, 0);
    g.end_sentence();
    let Some(prey) = targets::object(g, item, 0) else { return Ok(()) };
    fight::may_have_this_fight(g, item, prey)
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        undergrowth,
        "Undergrowth \u{2014} When this creature enters, choose up to one target creature you don't control with flying. This creature gets +X/+0 until end of turn, where X is the number of creature cards in your graveyard, then you may have this creature fight that creature.",
    )
    .targets(&[up_to_one(creature_you_dont_control_with_flying)])],
    ..oracle::KRAUL_HARPOONER
};
