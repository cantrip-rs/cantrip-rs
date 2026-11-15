use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature token" as a filter (CR 115.1, 111.1): a creature permanent that is a token, any controller's.
fn creature_token_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::creature_filter(q, asker, t) && matches!(t, Target::Object(id) if q.obj(id).token)
}

/// "Conduct Electricity deals 6 damage to target creature and 2 damage to up to one target creature token." One instruction
/// with two target slots (CR 608.2c, 115.1d): each hit is made only while its own target is still legal (CR 608.2b), and the
/// two are dealt together as one event (CR 120.3, 120.4b), so when both slots name the same creature it takes 8 at once.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut hits = Vec::new();
    if let Some(creature) = targets::object(g, item, 0) {
        hits.push((item.object, Target::Object(creature), 6));
    }
    if let Some(token) = g.state().spec_targets(item, 1).next().and_then(|slot| targets::object(g, item, slot)) {
        hits.push((item.object, Target::Object(token), 2));
    }
    g.damage_together(&hits);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::CREATURE.labelled("Creature to be dealt 6 damage"),
            up_to_one(creature_token_filter).labelled("Creature token to be dealt 2 damage"),
        ],
        effect,
    ),
    ..oracle::CONDUCT_ELECTRICITY
};
