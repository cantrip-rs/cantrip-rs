use cantrip_api::*;
use cantrip_oracle as oracle;

/// "X can't be greater than the number of snow lands you control." (CR 601.2b).
fn snow_land_limit(q: &dyn Query, asker: &Asker, _: &[Target]) -> u32 {
    q.state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| {
            q.chars(id).supertypes.snow
                && q.chars(id).is(CardType::Land)
                && q.controller(id) == asker.controller
        })
        .fold(0u32, |count, _| count + 1)
}

/// "If that player doesn't, destroy that creature at end of combat." (CR 608.2, 701.8a).
fn delayed_6(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_7 = {
        let q: &dyn Query = g;
        delayed_6_capture_0(q, item)
            .first()
            .copied()
            .into_iter()
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_7);
    Ok(())
}

/// "If that player doesn't, destroy that creature at end of combat." (CR 608.2).
fn delayed_6_capture_0(q: &dyn Query, item: &StackItem) -> Vec<ObjectId> {
    about(q, item, 0)
}

/// "Cast this spell only during combat before blockers are declared. X can't be greater than the number of snow lands you control. Choose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat." (CR 608.2, 611.2a, 118.1).
fn cast_spell_only_during_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let t_0 = announced(g, item, 0, true);
    for bound_0 in t_0
        .iter()
        .filter_map(|t| t.object())
        .collect::<Vec<ObjectId>>()
    {
        let payer_1 = g.controller(bound_0);
        let paid_1 = g.may_pay(payer_1, item.object, bound_0, cost("{1}"), &PAY_ONE)?;
        if paid_1 {
            {
                let to = Some(Target::None);
                let about = Some(bound_0);
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
                        Duration::EndOfCombat,
                    );
                }
            }
            g.end_sentence();
        } else {
            let payer_4 = g.controller(bound_0);
            let paid_4 = g.may_pay(payer_4, item.object, bound_0, cost("{2}"), &PAY_TWO)?;
            if paid_4 {
                g.end_sentence();
            } else {
                {
                    let about: Vec<ObjectId> = Some(bound_0).into_iter().collect::<Vec<ObjectId>>();
                    {
                        g.delayed_trigger_about(
                            DelayedWhen::Step {
                                step: Step::EndOfCombat,
                                on_turn_of: None,
                            },
                            item.object,
                            item.controller,
                            AbilityRef::delayed(item.ability.card, 0),
                            &about,
                        );
                    }
                }
                g.end_sentence();
            }
        }
        g.end_sentence();
    }
    Ok(())
}

/// "If that player doesn't, destroy that creature at end of combat." (CR 608.2).
fn about(q: &dyn Query, item: &StackItem, part: usize) -> Vec<ObjectId> {
    assert_eq!(part, 0, "one real-object list");
    q.event_objects(item).to_vec()
}

/// "Cast this spell only during combat before blockers are declared. X can't be greater than the number of snow lands you control. Choose X target attacking creatures. For each of those creatures, its controller may pay {1} or {2}. If that player doesn't, destroy that creature at end of combat. If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat." (CR 608.2).
fn announced(g: &dyn Game, item: &StackItem, spec: u8, legal: bool) -> Vec<Target> {
    (0..g.state().targets(item).len())
        .filter(|&i| g.state().spec(item, i) == spec && (!legal || g.target_legal(item, i)))
        .map(|i| g.state().target(item, i))
        .collect()
}

static PAY_ONE: Question = Question::new(
    "Pay",
    "pay {cost} to prevent combat damage to and by {subject}?",
    "If you decline, you may pay {2} instead",
    "Pay {cost}",
    "Decline",
);
static PAY_TWO: Question = Question::new(
    "Pay",
    "pay {cost} for {subject}?",
    "If you decline, it will be destroyed at end of combat",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[cantrip_mechanics::targets::ATTACKING_CREATURE.counted()],
        cast_spell_only_during_combat,
    ),
    delayed: &[
        delayed_trigger(
            delayed_6,
            "If that player doesn't, destroy that creature at end of combat.",
        ),
    ],
    granted_preventions: &[
        cantrip_mechanics::prevention::COMBAT_DAMAGE_DEALT_TO_AND_BY
            .text("If that player pays only {1}, prevent all combat damage that would be dealt to and dealt by that creature this combat."),
    ],
    cast_only_if: Some(cantrip_mechanics::combat::cast_only_before_blockers),
    x_rule: Some(XRule::AtMost(snow_land_limit)),
    ..oracle::WINTERS_CHILL
};
