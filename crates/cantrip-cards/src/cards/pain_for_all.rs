use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "Any other target" for an ability of an Aura: any target but the creature the Aura enchants (CR 115.1, 115.4), which is the
/// one dealing the damage.
fn any_target_but_the_enchanted_creature(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    q.obj(asker.source).attached_to.map(Target::Object) != Some(t) && (targets::ANY.filter)(q, asker, t)
}

/// "Enchanted creature deals damage equal to its power to any other target" (CR 120.3): the creature is the source, so its
/// keywords (deathtouch, lifelink, infect) count; its power is read as the ability resolves. If the creature has left the
/// battlefield by then, it deals the damage as it last existed, its power from its last-known information (CR 608.2h, 113.7a); the
/// ability still knows the creature the Aura was attached to when the Aura itself has gone too. Nothing happens if the target is no
/// longer legal (CR 608.2b).
fn enchanted_creature_deals_damage_equal_to_its_power(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to
        && g.target_legal(item, 0)
    {
        let power = targets::power_or_last_known(g, creature).max(0) as u32;
        g.damage(creature, item.target(0), power);
    }
    Ok(())
}


/// "It deals that much damage to each opponent" (CR 120.3, 608.2h): the enchanted creature, as the trigger saw it, deals the
/// amount - all damage dealt to it at once, however many sources - to every opponent still in the game together. The creature may
/// be gone by now; it deals the damage as it last existed (CR 113.7a), so a lifelinking creature still gains life.
fn it_deals_that_much_damage_to_each_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let amount = item.matched_damage_amount();
    let hits: Vec<(ObjectId, Target, u32)> = g.state().opponents(item.controller).map(|p| (creature, Target::Player(p), amount)).collect();
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            enchanted_creature_deals_damage_equal_to_its_power,
            "When this Aura enters, enchanted creature deals damage equal to its power to any other target.",
        )
        .targets(&[TargetSpec { filter: any_target_but_the_enchanted_creature, ..targets::ANY }]),
        triggered(
            triggers::ENCHANTED_CREATURE_DEALT_DAMAGE,
            it_deals_that_much_damage_to_each_opponent,
            "Whenever enchanted creature is dealt damage, it deals that much damage to each opponent.",
        )
        .sums_per_subject().damage_amount(),
    ],
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    ..oracle::PAIN_FOR_ALL
};
