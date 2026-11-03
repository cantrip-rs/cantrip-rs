use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Cast this spell only during an opponent's turn, before attackers are declared. Creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn." (CR 608.2).
fn cast_spell_only_during_opponents_if(q: &dyn Query, asker: &Asker) -> bool {
    q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .into_iter()
        .any(|bound_0: PlayerId| bound_0 == q.state().turn.active)
        && ((q.state().turn.step() as u8) < (Step::DeclareAttackers as u8)
            && !q
                .history()
                .this_turn()
                .iter()
                .any(|r| {
                    matches!(r.event, Event::StepBegan { step : Step::DeclareAttackers })
                }))
}

/// "Cast this spell only during an opponent's turn, before attackers are declared. Creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn." (CR 611.2).
fn lasting_rule_0(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.state().turn.active == q.controller(id)
}

/// "Cast this spell only during an opponent's turn, before attackers are declared. Creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn." (CR 608.2, 701.8a).
fn delayed_2(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let objects_3 = {
        let q: &dyn Query = g;
        q.state()
            .battlefield()
            .iter()
            .copied()
            .filter(|&id| {
                !(q.chars(id).has_subtype(q.words(item.object).subtype(Subtype::Wall)))
                    && q.chars(id).is(CardType::Creature)
                    && item.subject.player() == Some(q.controller(id))
                    && !q.attacked_this_turn(id)
                    && q.state().obj(id).controlled_since < q.state().turn.number
            })
            .collect::<Vec<ObjectId>>()
    };
    g.destroy_all(&objects_3);
    Ok(())
}

/// "Cast this spell only during an opponent's turn, before attackers are declared. Creatures the active player controls attack this turn if able. At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn." (CR 608.2, 611.2).
fn cast_spell_only_during_opponents(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::None, Duration::EndOfTurn);
    g.end_sentence();
    if let Some(player) = {
        let q: &dyn Query = g;
        Some(q.state().turn.active)
    } {
        g.delayed_trigger(
            DelayedWhen::Step {
                step: Step::End,
                on_turn_of: None,
            },
            item.object,
            item.controller,
            AbilityRef::delayed(item.ability.card, 0),
            Target::Player(player),
        );
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], cast_spell_only_during_opponents),
    delayed: &[
        delayed_trigger(
            delayed_2,
            "At the beginning of the next end step, destroy all non-Wall creatures that player controls that didn't attack this turn. Ignore this effect for each creature the player didn't control continuously since the beginning of the turn.",
        ),
    ],
    cast_only_if: Some(cast_spell_only_during_opponents_if),
    lasting_rules: &[
        LastingRule {
            text: "Creatures the active player controls attack this turn if able.",
            rule: LastingRuleKind::MustAttack,
            applies: lasting_rule_0,
        },
    ],
    ..oracle::SIRENS_CALL
};
