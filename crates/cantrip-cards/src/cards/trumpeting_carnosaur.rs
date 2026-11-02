use cantrip_api::*;
use cantrip_mechanics::{discover, triggers};
use cantrip_oracle as oracle;

/// "When this creature enters, discover 5." (CR 608.2).
fn discover_5(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    discover::discover(g, item.controller, 5)?;
    Ok(())
}

/// "{2}{R}, Discard this card: It deals 3 damage to target creature or planeswalker." (CR 115.1).
fn deals_3_damage_target_creature_target_0(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && ((q
        .chars(id).is(CardType::Creature) || q.chars(id).is(CardType::Planeswalker)))
    )
}

/// "{2}{R}, Discard this card: It deals 3 damage to target creature or planeswalker." (CR 608.2, 120.3).
fn deals_3_damage_target_creature(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let recipients_1 = t_0
        .get(0)
        .copied()
        .and_then(|t: Target| t.object())
        .into_iter()
        .map(Target::Object)
        .collect::<Vec<Target>>();
    let batch: Vec<(ObjectId, Target, u16)> = recipients_1
        .iter()
        .map(|t| (item.object, *t, 3u16))
        .collect();
    g.damage_together(&batch);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            discover_5,
            "When this creature enters, discover 5.",
        ),
        ability(
                cost("{2}{R}").and(DISCARD_THIS),
                deals_3_damage_target_creature,
                "{2}{R}, Discard this card: It deals 3 damage to target creature or planeswalker.",
            )
            .targets(&[target(deals_3_damage_target_creature_target_0)])
            .from(ZoneKind::Hand),
    ],
    ..oracle::TRUMPETING_CARNOSAUR
};
