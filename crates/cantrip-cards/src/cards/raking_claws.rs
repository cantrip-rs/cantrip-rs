use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target creature gains double strike until end of turn." (CR 115.1).
fn target_creature_gains_double_strike_target_0(
    q: &dyn Query,
    _: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if cantrip_mechanics::targets::permanent(q, id) && (q
        .chars(id).is(CardType::Creature))
    )
}

/// "Target creature gains double strike until end of turn." (CR 608.2, 611.2a).
fn target_creature_gains_double_strike(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::DoubleStrike]));
    instruction(
        g,
        |g| {
            if let Some(object) = objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

/// "Cycling {2}" (CR 608.2, 121.1).
fn cycling_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[target(target_creature_gains_double_strike_target_0)],
        target_creature_gains_double_strike,
    ),
    abilities: &[
        ability(cost("{2}").and(DISCARD_THIS), cycling_2, "Cycling {2}")
            .cycling()
            .from(ZoneKind::Hand),
    ],
    ..oracle::RAKING_CLAWS
};
