use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{G}{G}{G}{G}, {T}: Target creature gains forestwalk until end of turn." (CR 608.2, 611.2a).
fn target_creature_gains_forestwalk(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = targets::object(g, item, 0);
    let kind_0_0_0 = {
        let q: &dyn Query = g;
        EffectKind::AddKeywords(
            Keyword::landwalk(q.words(item.object).subtype(Subtype::Forest))
                .map_or(Keywords::NONE, |walk| Keywords::of(&[walk])),
        )
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

/// "{T}: Target creature loses forestwalk until end of turn." (CR 608.2, 611.2a).
fn target_creature_loses_forestwalk(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_1 = targets::object(g, item, 0);
    let kind_1_0_0 = {
        let q: &dyn Query = g;
        EffectKind::LoseKeywords(
            Keyword::landwalk(q.words(item.object).subtype(Subtype::Forest))
                .map_or(Keywords::NONE, |walk| Keywords::of(&[walk])),
        )
    };
    instruction(
        g,
        |g| {
            if let Some(object) = objects_1 {
                g.add_effect(item.object, object, kind_1_0_0, Duration::EndOfTurn);
            }
        },
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{G}{G}{G}{G}").and(TAP),
                target_creature_gains_forestwalk,
                "{G}{G}{G}{G}, {T}: Target creature gains forestwalk until end of turn.",
            )
            .targets(&[targets::CREATURE]),
        ability(
                TAP,
                target_creature_loses_forestwalk,
                "{T}: Target creature loses forestwalk until end of turn.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::SCARWOOD_HAG
};
