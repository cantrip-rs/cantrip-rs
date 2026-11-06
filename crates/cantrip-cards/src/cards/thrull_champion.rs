use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets};
use cantrip_oracle as oracle;

/// "Thrull creatures get +1/+1." (CR 604.1).
fn thrull_creatures_get_1_1_affects(
    q: &dyn LayerQuery,
    source: ObjectId,
    _: ObjectId,
    so_far: &Chars,
) -> bool {
    let asker = &Asker::new(source, q.controller(source));
    (*so_far).has_subtype(q.words(asker.source).subtype(Subtype::Thrull))
        && (*so_far).is(CardType::Creature)
}

/// "{T}: Gain control of target Thrull for as long as you control this creature." (CR 115.1).
fn gain_control_target_thrull_as_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    targets::permanent_of_any_subtype(q, asker, t, &[Subtype::Thrull])
}

/// "{T}: Gain control of target Thrull for as long as you control this creature." (CR 608.2, 611.2a).
fn gain_control_target_thrull_as(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
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
            if let Some(object) = objects_0 {
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
        ability(
                TAP,
                gain_control_target_thrull_as,
                "{T}: Gain control of target Thrull for as long as you control this creature.",
            )
            .targets(&[target(gain_control_target_thrull_as_target_0)]),
    ],
    statics: &[
        static_ability(
            Layer::PtModify,
            thrull_creatures_get_1_1_affects,
            statics::plus_pt_saturating::<1, 1>,
            "Thrull creatures get +1/+1.",
        ),
    ],
    lasting: &[effects::YOU_CONTROL_SOURCE],
    ..oracle::THRULL_CHAMPION
};
