use cantrip_api::*;
use cantrip_mechanics::{blight, targets};
use cantrip_oracle as oracle;

/// "Enchanted creature can't attack" (CR 508.1c, 303.4): a rule of the Aura's, asked of each creature that could be declared
/// an attacker; false for the one this Aura is attached to.
fn enchanted_cant_attack(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    q.obj(source).attached_to != Some(attacker)
}

/// "Enchanted creature can't ... block" (CR 509.1b).
fn enchanted_cant_block(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    q.obj(source).attached_to != Some(blocker)
}

/// "{1}{W}, Blight 1, Sacrifice this Aura: Exile enchanted creature." The Aura is sacrificed as a cost, so the creature is the
/// one it was attached to as it left (CR 608.2h); a creature that has left the battlefield since is not exiled (CR 400.7).
fn exile_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let state = g.state();
    let host = state.obj(item.object).attached_to.filter(|&h| state.obj(h).became.is_none() && state.obj(h).zone.is(ZoneKind::Battlefield));
    host.map(|host| g.exile(host));
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{1}{W}").and(blight::cost(1)).and(SACRIFICE_THIS),
        exile_enchanted,
        "{1}{W}, Blight 1, Sacrifice this Aura: Exile enchanted creature.",
    )],
    attack_rules: &[enchanted_cant_attack],
    block_rules: &[enchanted_cant_block],
    ..oracle::SPIRAL_INTO_SOLITUDE
};
