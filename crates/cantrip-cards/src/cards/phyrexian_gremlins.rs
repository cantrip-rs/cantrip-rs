use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "{T}: Tap target artifact. It doesn't untap during its controller's untap step for as long as this creature remains tapped." (CR 608.2, 611.2a, 701.26a).
fn tap_target_artifact_doesnt_untap(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    effects::tap_targets(g, item)?;
    g.end_sentence();
    let objects_1 = {
        let q: &dyn Query = g;
        t_0.get(0)
            .copied()
            .and_then(|t: Target| t.object())
            .into_iter()
            .filter(|&o| is_live(q, o))
            .collect::<Vec<ObjectId>>()
    };
    let kinds_1 = {
        let mut all = Vec::new();
        for &object in &objects_1 {
            all.push((object, EffectKind::DoesntUntap));
        }
        all
    };
    instruction(
        g,
        |g| {
            for &(object, kind) in &kinds_1 {
                g.add_effect(item.object, object, kind, Duration::WhileSourceTapped);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                tap_target_artifact_doesnt_untap,
                "{T}: Tap target artifact. It doesn't untap during its controller's untap step for as long as this creature remains tapped.",
            )
            .targets(&[targets::ARTIFACT]),
    ],
    may_not_untap: true,
    ..oracle::PHYREXIAN_GREMLINS
};
