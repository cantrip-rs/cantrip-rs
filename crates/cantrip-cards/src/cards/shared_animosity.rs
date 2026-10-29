use cantrip_api::*;
use cantrip_mechanics::{pump, statics};
use cantrip_mechanics::triggers::CREATURE_YOU_CONTROL_ATTACKS;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// It gets +1/+0 until end of turn for each other attacking creature that shares a creature type with it. Counted as
/// the ability resolves (CR 608.2h), and the bonus is fixed for the turn (CR 611.2c).
fn bonus(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(attacker) = item.subject else { return Ok(()) };
    if !targets::permanent(g, attacker) {
        return Ok(());
    }
    let mine = g.chars(attacker).subtypes;
    let others: Vec<ObjectId> = g.state().attackers.iter().map(|a| a.id).filter(|&id| id != attacker).collect();
    let shares = |g: &dyn Game, id: ObjectId| {
        let c = g.chars(id);
        c.is(CardType::Creature) && statics::shares_creature_type(mine, c.subtypes)
    };
    let count = others.into_iter().filter(|&id| shares(&*g, id)).count() as i16;
    if count > 0 {
        pump(g, item.object, attacker, count, 0);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(CREATURE_YOU_CONTROL_ATTACKS, bonus, "Whenever a creature you control attacks, it gets +1/+0 until end of turn for each other attacking creature that shares a creature type with it.")],
    ..oracle::SHARED_ANIMOSITY
};
