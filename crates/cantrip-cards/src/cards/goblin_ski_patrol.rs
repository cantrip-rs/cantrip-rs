use cantrip_api::*;
use cantrip_oracle as oracle;

/// "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain." (CR 608.2, 701.21a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let controller = g.controller(item.object);
    g.sacrifice(controller, &[item.object]);
    Ok(())
}

/// "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain." (CR 608.2, 611.2a).
fn creature_gets_2_0_gains(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_0_1_0 = EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying]));
    instruction(
        g,
        |g| {
            for &object in &objects_0 {
                g.add_effect(
                    item.object,
                    object,
                    EffectKind::Pump(2, 0),
                    Duration::Indefinite,
                );
                g.add_effect(item.object, object, kind_0_1_0, Duration::Indefinite);
            }
        },
    );
    g.end_sentence();
    g.delayed_trigger(
        DelayedWhen::Step {
            step: Step::End,
            on_turn_of: None,
        },
        item.object,
        item.controller,
        AbilityRef::delayed(item.ability.card, 0),
        Target::None,
    );
    Ok(())
}

/// "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain." (CR 608.2).
fn creature_gets_2_0_gains_if(q: &dyn Query, asker: &Asker) -> bool {
    q.permanents_of(asker.controller).into_iter().any(|id| {
        q.chars(id).supertypes.snow
            && q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Mountain))
    })
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{1}{R}"),
                creature_gets_2_0_gains,
                "{1}{R}: This creature gets +2/+0 and gains flying. Its controller sacrifices it at the beginning of the next end step. Activate only once and only if you control a snow Mountain.",
            )
            .once()
            .condition(creature_gets_2_0_gains_if),
    ],
    delayed: &[
        delayed_trigger(
            delayed_2,
            "Its controller sacrifices it at the beginning of the next end step.",
        ),
    ],
    ..oracle::GOBLIN_SKI_PATROL
};
