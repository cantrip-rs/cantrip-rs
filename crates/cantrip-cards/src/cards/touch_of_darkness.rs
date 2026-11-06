use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "One or more target creatures become black until end of turn." (CR 608.2, 611.2a).
fn one_more_target_creatures_become(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    let objects_0 = t_0.iter().filter_map(|t| t.object()).collect::<Vec<ObjectId>>();
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::SetColors(q.words(item.object).colors(ColorSet(4)))
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

/// "One or more target creatures become black until end of turn." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| {
            g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i))
        })
        .map(|i| g.state().target(item, i))
        .collect()
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::CREATURE.between(1, TargetSpec::ANY_NUMBER)],
        one_more_target_creatures_become,
    ),
    ..oracle::TOUCH_OF_DARKNESS
};
