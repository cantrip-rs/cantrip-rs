use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, Halfdane's base power and toughness become equal to the power and toughness of target creature other than Halfdane until the end of your next upkeep." (CR 608.2, 611.2a).
fn halfdanes_base_power_toughness_become(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let duration_0 = Duration::UntilEndOfNextStepOf {
        step: Step::Upkeep,
        player: Some(item.controller),
    };
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::SetPt(
            t_0
                .get(0)
                .copied()
                .and_then(|t: Target| t.object())
                .map_or(0, |o: ObjectId| i64::from(q.chars(o).power.unwrap_or(0)))
                as i32,
            t_0
                .get(0)
                .copied()
                .and_then(|t: Target| t.object())
                .map_or(0, |o: ObjectId| i64::from(q.chars(o).toughness.unwrap_or(0)))
                as i32,
        )
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(item.object, object, kind_0_0_0, duration_0);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
                triggers::YOUR_UPKEEP,
                halfdanes_base_power_toughness_become,
                "At the beginning of your upkeep, Halfdane's base power and toughness become equal to the power and toughness of target creature other than Halfdane until the end of your next upkeep.",
            )
            .targets(&[targets::OTHER_CREATURE]),
    ],
    ..oracle::HALFDANE
};
