use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::ANOTHER_CREATURE_YOU_CONTROL_ENTERS;
use cantrip_oracle as oracle;

/// "Spells your opponents cast that target this creature cost an additional 3 life to cast" (CR 118.8, 601.2f): an
/// additional cost of the total, determined once the spell's targets are announced, so a spell that has this creature
/// among them - in any slot, however many times - costs 3 life more, and one that does not costs nothing more. It is
/// paid with the rest of the cost (CR 601.2h); a player with less life than the total asks can't cast the spell that way
/// (CR 119.4). Abilities, and spells of its controller, are not affected.
fn opponents_spells_targeting_this_cost_three_life(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, targets: &[Target]) -> CostDelta {
    if !matches!(costed, Costed::Spell(_)) || !q.state().opponents(q.controller(source)).any(|p| p == player) {
        return CostDelta::NONE;
    }
    if targets.contains(&Target::Object(source)) { CostDelta::life(3) } else { CostDelta::NONE }
}

/// "This creature deals damage equal to that creature's power to any target" (CR 120.3): the entered creature's power as
/// the ability resolves, or as it last existed on the battlefield once it has left (CR 608.2h); this creature is the
/// source, as it last existed if it has left (CR 113.7a). A power of 0 or less deals no damage (CR 120.8).
fn damage_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(entered) = item.subject else { return Ok(()) };
    let power = g.chars(entered).power.unwrap_or(0);
    if power > 0 {
        g.damage(item.object, item.target(0), i64::from(power as u16));
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cost_changes: &[opponents_spells_targeting_this_cost_three_life],
    abilities: &[triggered(ANOTHER_CREATURE_YOU_CONTROL_ENTERS, damage_equal_to_its_power, "Whenever another creature you control enters, this creature deals damage equal to that creature's power to any target.")
        .targets(&[targets::ANY])],
    ..oracle::TERROR_OF_THE_PEAKS
};
