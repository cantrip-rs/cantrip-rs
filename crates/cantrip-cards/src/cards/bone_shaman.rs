use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Creatures dealt damage by this creature this turn can't be regenerated this turn." (CR 604.1).
fn granted_1_0_rule(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
        && ({
            let history_candidate = id;
            q
                .history()
                .this_turn()
                .iter()
                .filter_map(|r: &Record| -> Option<Fired> {
                    let event = &r.event;
                    if let Event::Damage { source, target, .. } = event {
                        let source = *source;
                        let subject = *target;
                        if q.state().obj(source).zone.is(ZoneKind::Battlefield)
                            && asker.source == source
                            && matches!(
                                * target, Target::Object(o) if history_candidate == o
                            )
                        {
                            return Fired::about(subject);
                        }
                    }
                    None
                })
                .count() as i64
        }) >= 1
}

/// "{B}: Until end of turn, this creature gains "Creatures dealt damage by this creature this turn can't be regenerated this turn."" (CR 608.2, 611.2a).
fn creature_gains_creatures_dealt_damage(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let objects_0 = Some(item.object).into_iter().collect::<Vec<ObjectId>>();
    let kind_0_0_0 = EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0));
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
    abilities: &[
        ability(
            cost("{B}"),
            creature_gains_creatures_dealt_damage,
            "{B}: Until end of turn, this creature gains \"Creatures dealt damage by this creature this turn can't be regenerated this turn.\"",
        ),
    ],
    delayed: &[
        granted_regeneration_ban(
            granted_1_0_rule,
            "Creatures dealt damage by this creature this turn can't be regenerated this turn.",
        ),
    ],
    ..oracle::BONE_SHAMAN
};
