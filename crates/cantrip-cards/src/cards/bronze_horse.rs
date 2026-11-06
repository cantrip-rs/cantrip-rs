use cantrip_api::*;
use cantrip_oracle as oracle;

/// "As long as you control another creature, prevent all damage that would be dealt to this creature by spells that target it." (CR 615.1).
fn as_long_as_control_another_applies(
    q: &dyn Query,
    asker: &Asker,
    e: &DamageEvent,
) -> bool {
    {
        let (source, target) = (e.source, &e.target);
        if q.state().obj(source).zone.is(ZoneKind::Stack)
            && q
                .state()
                .stack
                .iter()
                .find(|i| i.kind == StackKind::Spell && i.object == source)
                .is_some_and(|i| {
                    q
                        .state()
                        .targets(i)
                        .iter()
                        .any(|t| {
                            t
                                .object()
                                .is_some_and(|o| {
                                    Some(asker.source)
                                        .into_iter()
                                        .collect::<Vec<ObjectId>>()
                                        .contains(&o)
                                })
                        })
                }) && matches!(* target, Target::Object(o) if asker.source == o)
            && q
                .state()
                .battlefield()
                .iter()
                .copied()
                .filter(|&id| {
                    q.controller(id)
                        == Some(asker.controller).unwrap_or(PlayerId(u8::MAX))
                        && (!(asker.source == id) && q.chars(id).is(CardType::Creature))
                })
                .count() >= 1
        {
            return true;
        }
    }
    false
}

pub static CARD: CardDef = CardDef {
    preventions: &[
        PreventionDef {
            hint: "As long as you control another creature, prevent all damage that would be dealt to this creature by spells that target it.",
            applies: as_long_as_control_another_applies,
            amount: PreventionAmount::All,
        },
    ],
    ..oracle::BRONZE_HORSE
};
