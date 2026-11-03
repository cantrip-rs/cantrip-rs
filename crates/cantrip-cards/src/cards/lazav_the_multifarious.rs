use cantrip_api::*;
use cantrip_mechanics::{triggers, surveil};
use cantrip_oracle as oracle;

/// "When Lazav enters, surveil 1." (CR 608.2).
fn surveil_1(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    surveil(g, item.controller, item.object, 1)?;
    Ok(())
}

/// "{X}: Lazav becomes a copy of target creature card in your graveyard with mana value X, except its name is Lazav, the Multifarious, it's legendary in addition to its other types, and it has this ability." (CR 115.1).
fn lazav_becomes_copy_target_creature_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if (q.state().obj(id).zone.kind() == ZoneKind::Graveyard)
        && ! q.state().obj(id).token && ((q.chars(id).is(CardType::Creature) && q
        .announced_x().map_or(true, | x | (i64::from(q.mana_value(id))) ==
        (i64::from(x))) && (q.state().obj(id).zone.kind() == ZoneKind::Graveyard && asker
        .controller == q.state().obj(id).owner)))
    )
}

/// "{X}: Lazav becomes a copy of target creature card in your graveyard with mana value X, except its name is Lazav, the Multifarious, it's legendary in addition to its other types, and it has this ability." (CR 608.2).
fn lazav_becomes_copy_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_1 = {
        let q: &dyn Query = g;
        Some(item.object)
            .into_iter()
            .filter(|&o| q.state().obj(o).became.is_none())
            .collect::<Vec<ObjectId>>()
    };
    let original_1 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .collect::<Vec<_>>();
    let values_1 = {
        let q: &dyn Query = g;
        original_1
            .first()
            .copied()
            .map(|original: ObjectId| {
                q
                    .copiable_values(original)
                    .named("Lazav, the Multifarious")
                    .legendary()
                    .with_ability(item.ability)
            })
    };
    let Some(values) = values_1 else { return Ok(()) };
    instruction(
        g,
        |g| {
            for &object in &objects_1 {
                g.become_copy(item.object, object, values, Duration::Indefinite);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, surveil_1, "When Lazav enters, surveil 1."),
        ability(
                cost("{X}"),
                lazav_becomes_copy_target_creature,
                "{X}: Lazav becomes a copy of target creature card in your graveyard with mana value X, except its name is Lazav, the Multifarious, it's legendary in addition to its other types, and it has this ability.",
            )
            .targets(&[target(lazav_becomes_copy_target_creature_target_0)])
            .targets_after_x(),
    ],
    ..oracle::LAZAV_THE_MULTIFARIOUS
};
