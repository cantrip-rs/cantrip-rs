use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you." (CR 115.1).
fn exile_creature_target_creature_without_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    (cantrip_mechanics::targets::CREATURE.filter)(q, asker, t) && matches!(
        t, Target::Object(id) if ((! (q.chars(id).keywords
        .has(Keyword::Flying)) && q.state().attackers.iter().any(| a | a.id == id && {
        let defender = & a.target; match * defender { Target::Player(p) => asker
        .controller == p, _ => false } })))
    )
}

/// "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you." (CR 608.2, 701.13a).
fn exile_creature_target_creature_without(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let mut all: Vec<ObjectId> = Vec::new();
        if !all.contains(&item.object) {
            all.push(item.object);
        }
        for id in t_0
            .get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .collect::<Vec<_>>()
        {
            if !all.contains(&id) {
                all.push(id);
            }
        }
        all
    };
    g.exile_all(&objects_0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{R}{G}").and(TAP),
                exile_creature_target_creature_without,
                "{1}{R}{G}, {T}: Exile this creature and target creature without flying that's attacking you.",
            )
            .targets(&[target(exile_creature_target_creature_without_target_0)]),
    ],
    ..oracle::GIANT_TRAP_DOOR_SPIDER
};
