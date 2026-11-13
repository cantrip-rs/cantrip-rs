use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "{G}{G}, {T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature." (CR 608.2, 120.3).
fn creature_deals_damage_equal_power(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    fight::bite(g, item.object, target);
    g.end_sentence();
    fight::bite(g, target, item.object);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{G}{G}").and(TAP),
                creature_deals_damage_equal_power,
                "{G}{G}, {T}: This creature deals damage equal to its power to target creature. That creature deals damage equal to its power to this creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::TRACKER
};
