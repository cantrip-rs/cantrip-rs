use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "One or more target creatures become blue until end of turn." (CR 608.2, 611.2a).
fn one_more_target_creatures_become(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects = targets::legal_objects(g, item);
    let colors = g.words(item.object).colors(ColorSet::of(Color::Blue));
    instruction(g, |g| {
        for object in objects {
            g.add_effect(item.object, object, EffectKind::SetColors(colors), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE.between(1, TargetSpec::ANY_NUMBER)],
        one_more_target_creatures_become,
    ),
    ..oracle::SEA_KINGS_BLESSING
};
