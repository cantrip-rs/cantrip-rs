use cantrip_api::*;
use cantrip_mechanics::{targets, tokens};
use cantrip_oracle as oracle;

/// "Enchanted creature can't attack" (CR 508.1c): the creature this Aura is attached to.
fn enchanted_creature_cant_attack(q: &dyn Query, source: ObjectId, attacker: ObjectId) -> bool {
    q.attached_to(source) != Some(attacker)
}

/// "Enchanted creature can't ... block" (CR 509.1b).
fn enchanted_creature_cant_block(q: &dyn Query, source: ObjectId, blocker: ObjectId, _: ObjectId) -> bool {
    q.attached_to(source) != Some(blocker)
}

/// "Exile enchanted creature. You create a 1/1 black Ninja creature token": the Aura was sacrificed as a cost, so the creature
/// is the one it was last attached to (CR 608.2h), exiled only if it is still on the battlefield. The token is made either way.
fn exile_enchanted_creature_and_create_a_ninja(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to
        && targets::permanent(g, creature)
    {
        g.exile(creature);
    }
    g.end_sentence();
    tokens::create(g, item.controller, &tokens::NINJA, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[ability(
        cost("{5}").and(Cost { sacrifice_self: true, ..Cost::NONE }),
        exile_enchanted_creature_and_create_a_ninja,
        "{5}, Sacrifice this Aura: Exile enchanted creature. You create a 1/1 black Ninja creature token. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    attack_rules: &[enchanted_creature_cant_attack],
    block_rules: &[enchanted_creature_cant_block],
    ..oracle::UNEASY_ALLIANCE
};

pub static EXTRA: &[&CardDef] = &[&tokens::NINJA];
