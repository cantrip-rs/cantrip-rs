use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap all creatures blocking target attacking creature. Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it." (CR 615.1).
fn made_2_applies(
    q: &dyn Query,
    _: &Asker,
    affected: ObjectId,
    e: &DamageEvent,
) -> bool {
    {
        let (source, combat) = (e.source, e.combat);
        if combat && !q.state().obj(source).zone.is(ZoneKind::Stack)
            && (affected == source
                || (q.chars(source).is(CardType::Creature)
                    && {
                        let attacker = Some(affected);
                        q.state()
                            .blocks
                            .iter()
                            .any(|b| b.blocker == source && Some(b.attacker) == attacker)
                    }))
        {
            return true;
        }
    }
    false
}

/// "Tap all creatures blocking target attacking creature. Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it." (CR 608.2, 611.2a, 701.26a).
fn tap_all_creatures_blocking_target(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    let t_0 = vec![g.state().target(item, 0)];
    let objects_0 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                q.chars(id).is(CardType::Creature)
                    && {
                        let attacker = t_0
                            .get(0)
                            .copied()
                            .and_then(|t: Target| t.object());
                        q.state()
                            .blocks
                            .iter()
                            .any(|b| b.blocker == id && Some(b.attacker) == attacker)
                    }
            })
            .collect::<Vec<ObjectId>>()
    };
    g.tap_together(&objects_0);
    g.end_sentence();
    {
        let to = Some(Target::None);
        let about = t_0.get(0).copied().and_then(|t: Target| t.object());
        if let (Some(to), Some(about)) = (to, about) {
            g.add_effect(
                item.object,
                about,
                EffectKind::Prevention {
                    card: item.ability.card,
                    index: 0,
                    to,
                    shield: Shield::All,
                },
                Duration::EndOfTurn,
            );
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[targets::ATTACKING_CREATURE],
        tap_all_creatures_blocking_target,
    ),
    granted_preventions: &[
        GrantedPreventionDef {
            hint: "Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it.",
            damage: "damage",
            of_it: None,
            applies: made_2_applies,
            then: None,
            text: "Prevent all combat damage that would be dealt this turn by that creature and each creature blocking it.",
        },
    ],
    ..oracle::FEINT
};
