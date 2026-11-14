use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Must be blocked this turn if able" is about every creature its controller controls, as a rule of the game and not as a
/// characteristic: a creature that comes under their control later in the turn is taken in (CR 509.1c, 611.2c).
fn lasting_rule_0(q: &dyn Query, asker: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature) && q.controller(id) == asker.controller
}

/// "Each creature you control gets +3/+3 until end of turn and must be blocked this turn if able." Only the creatures you control
/// as it resolves get +3/+3 (CR 611.2c); the requirement reaches the ones that arrive later.
fn pump_and_must_be_blocked(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::pump_creatures_you_control(g, item, 3, 3);
    g.end_sentence();
    g.add_lasting_rule(item, 0, Target::None, Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], pump_and_must_be_blocked),
    lasting_rules: &[LastingRule {
        text: "Each creature you control must be blocked this turn if able.",
        rule: LastingRuleKind::MustBeBlocked,
        applies: lasting_rule_0,
    }],
    ..oracle::JORAGA_INVOCATION
};
