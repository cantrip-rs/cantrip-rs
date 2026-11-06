use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "{2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn." (CR 115.1).
fn untap_target_attacking_creature_control_target_0(
    q: &dyn Query,
    asker: &Asker,
    t: Target,
) -> bool {
    matches!(
        t, Target::Object(id) if targets::permanent(q, id) && q.is_attacking(id)
        && q.chars(id).is(CardType::Creature) && asker.controller == q.controller(id)
    )
}

/// "{2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn." (CR 615.1).
fn made_2_applies(
    _: &dyn Query,
    _: &Asker,
    affected: ObjectId,
    e: &DamageEvent,
) -> bool {
    e.combat && (e.source == affected || e.target == Target::Object(affected))
}

/// "{2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn." (CR 608.2, 611.2a, 701.26b).
fn untap_target_attacking_creature_control(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let Some(object) = targets::object(g, item, 0) else { return Ok(()) };
    instruction(g, |g| { g.untap(object); });
    g.end_sentence();
    g.add_effect(item.object, object, EffectKind::Prevention {
        card: item.ability.card, index: 0, to: Target::None, shield: Shield::All,
    }, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                cost("{2}").and(TAP),
                untap_target_attacking_creature_control,
                "{2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn.",
            )
            .targets(&[target(untap_target_attacking_creature_control_target_0)]),
    ],
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "{2}, {T}: Untap target attacking creature you control. Prevent all combat damage that would be dealt to and dealt by that creature this turn.",
            damage: "combat damage",
            of_it: Some("to and dealt by it"),
            applies: made_2_applies,
            then: None,
            text: "Prevent all combat damage that would be dealt to and dealt by that creature this turn.",
        },
    ],
    ..oracle::EBONY_HORSE
};
