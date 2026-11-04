use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "One or more target creatures become red until end of turn." (CR 608.2, 611.2a).
fn one_more_target_creatures_become(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::legal_objects(g, item);
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::SetColors(q.words(item.object).colors(ColorSet::of(Color::Red)))
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE.between(1, TargetSpec::ANY_NUMBER)],
        one_more_target_creatures_become,
    ),
    ..oracle::DWARVEN_SONG
};
