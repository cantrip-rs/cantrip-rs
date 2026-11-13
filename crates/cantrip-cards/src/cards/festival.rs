use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Cast this spell only during an opponent's upkeep." (CR 601.3).
fn cast_spell_only_during_opponents_if(q: &dyn Query, asker: &Asker) -> bool {
    q
        .state()
        .opponents(asker.controller)
        .collect::<Vec<PlayerId>>()
        .into_iter()
        .any(|bound_0: PlayerId| bound_0 == q.state().turn.active)
        && q.state().turn.step() == Step::Upkeep
}

/// "Cast this spell only during an opponent's upkeep. Creatures can't attack this turn." (CR 611.2c).
fn lasting_rule_0(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// "Cast this spell only during an opponent's upkeep. Creatures can't attack this turn." (CR 608.2, 611.2c).
fn cast_spell_only_during_opponents(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::None, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], cast_spell_only_during_opponents),
    cast_only_if: Some(cast_spell_only_during_opponents_if),
    lasting_rules: &[
        LastingRule {
            text: "Creatures can't attack this turn.",
            rule: LastingRuleKind::CantAttack,
            applies: lasting_rule_0,
        },
    ],
    ..oracle::FESTIVAL
};
