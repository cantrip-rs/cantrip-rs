use cantrip_api::*;
use cantrip_mechanics::{damage_target, targets};
use cantrip_oracle as oracle;

/// "{T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down." (CR 608.2, 120.3).
fn creature_deals_damage_any_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let power = g.cost_objects(item).first().map_or(0, |&o| g.chars(o).power.unwrap_or(0));
    let damage = (i32::from(power).div_euclid(2)).max(0) as u32;
    damage_target(g, item, 0, damage);
    Ok(())
}

/// "{T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down." (CR 608.2).
fn creature_deals_damage_any_target_2(
    q: &dyn Query,
    asker: &Asker,
    id: ObjectId,
) -> bool {
    (q.chars(id).colors.has(q.words(asker.source).color(Color::Red))
        || q.chars(id).colors.has(q.words(asker.source).color(Color::White)))
        && q.chars(id).is(CardType::Creature)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP.and(sacrifice(1, creature_deals_damage_any_target_2)),
                creature_deals_damage_any_target,
                "{T}, Sacrifice a red or white creature: This creature deals damage to any target equal to half the sacrificed creature's power, rounded down.",
            )
            .targets(&[targets::ANY]),
    ],
    ..oracle::FREYALISE_SUPPLICANT
};
