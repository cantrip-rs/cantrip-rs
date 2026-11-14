use cantrip_api::*;
use cantrip_mechanics::{combat, targets};
use cantrip_oracle as oracle;

/// "{T}: Target creature can't be blocked by Walls this turn." (CR 608.2, 611.2a).
fn target_creature_cant_be_blocked(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::BlockRestriction {
            card: {
                let o = q.state().obj(item.object);
                o.copied_card().unwrap_or(o.card)
            },
            index: 0,
        }
    };
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

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP,
                target_creature_cant_be_blocked,
                "{T}: Target creature can't be blocked by Walls this turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    lasting_block_rules: &[
        LastingBlockRule {
            text: "Can't be blocked by Walls",
            allows: combat::cant_be_blocked_by_walls,
        },
    ],
    ..oracle::TOWER_OF_COIREALL
};
