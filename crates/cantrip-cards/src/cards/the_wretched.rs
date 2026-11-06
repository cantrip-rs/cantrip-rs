use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At end of combat, gain control of all creatures blocking this creature for as long as you control this creature." (CR 608.2).
fn lasting_0(q: &dyn Query, e: &ContinuousEffect) -> bool {
    is_live(q, e.source) && q.is_on_battlefield(e.source)
        && q.controller(e.source) == e.controller
}

/// "At end of combat, gain control of all creatures blocking this creature for as long as you control this creature." (CR 608.2, 611.2a).
fn gain_control_all_creatures_blocking(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && q
                        .state()
                        .blocks
                        .iter()
                        .any(|b| b.blocker == id && b.attacker == item.object)
            })
            .collect::<Vec<ObjectId>>()
    };
    let duration_0 = {
        let q: &dyn Query = g;
        Duration::AsLongAs(
            {
                let o = q.state().obj(item.object);
                o.copied_card().unwrap_or(o.card)
            },
            0,
        )
    };
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Control(item.controller),
                    duration_0,
                );
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::END_OF_COMBAT,
            gain_control_all_creatures_blocking,
            "At end of combat, gain control of all creatures blocking this creature for as long as you control this creature.",
        ),
    ],
    lasting: &[
        Lasting {
            text: "you control {source}",
            holds: lasting_0,
        },
    ],
    ..oracle::THE_WRETCHED
};
