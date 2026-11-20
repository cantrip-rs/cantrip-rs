use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Whenever an instant or sorcery spell you control that targets only a single creature deals damage to that creature" (CR 120.2,
/// 603.2, 115.1, 601.2c): the spell is the damage's source, an instant or sorcery its controller controls (the spell on the stack, or
/// as it last was when it has left it, CR 608.2h), and every target it was announced with is the one creature, however many times it
/// is chosen. A spell with a player or a second object as a target, or with no target, does not qualify, nor does damage the spell
/// deals to something else or to a creature it did not target. Damage of 0 is never dealt (CR 120.8), so no event is raised for it
/// (Fireball with X = 0 is the scenario). Each damage event is its own trigger.
fn your_spell_deals_damage_to_the_one_creature_it_targets(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Damage { source, target: Target::Object(hurt), .. } = event else { return None };
    let (item, first) = targets::spell_targeting_only_one_object(q, *source)?;
    let chars = q.chars(*source);
    if item.controller != me.controller || !(chars.is(CardType::Instant) || chars.is(CardType::Sorcery)) || first != *hurt || !q.chars(first).is(CardType::Creature) {
        return None;
    }
    Fired::about(Target::Object(*source))
}

fn damage_each_opponent_that_much(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_opponent(g, item, item.matched_damage_amount());
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, your_spell_deals_damage_to_the_one_creature_it_targets),
        damage_each_opponent_that_much,
        "Whenever an instant or sorcery spell you control that targets only a single creature deals damage to that creature, Imodane deals that much damage to each opponent.",
    )
    .damage_amount()],
    ..oracle::IMODANE_THE_PYROHAMMER
};
