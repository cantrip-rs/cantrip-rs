use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Each creature" (CR 611.2c): every creature on the battlefield as the rule is read, whoever controls it and whenever it
/// arrived (the rulings).
fn each_creature(q: &dyn Query, _: &Asker, _: Target, id: ObjectId) -> bool {
    q.chars(id).is(CardType::Creature)
}

/// I - "Until your next turn, each creature attacks each combat if able and attacks a player other than you if able." A rule
/// about an open set (CR 611.2c), with goad's requirements but no creature goaded (the rulings).
fn carnage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_lasting_rule(item, 0, Target::None, Duration::UntilNextTurnOf(item.controller));
    Ok(())
}

/// II - "Add {R}{R}{R}." (CR 106.4): the chapter triggers in the precombat main phase (CR 714.3c), so the mana can be spent
/// there.
fn add_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 3);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, carnage, "I \u{2014} Until your next turn, each creature attacks each combat if able and attacks a player other than you if able."),
        chapter(2, add_red, "II \u{2014} Add {R}{R}{R}.").produces_mana(),
        chapter(3, effects::damage_each_opponent_n::<5>, "III \u{2014} This Saga deals 5 damage to each opponent."),
    ],
    lasting_rules: &[LastingRule {
        text: "Until your next turn, each creature attacks each combat if able and attacks a player other than you if able.",
        rule: LastingRuleKind::MustAttackPlayerOtherThanYou,
        applies: each_creature,
    }],
    ..oracle::MAXIMUM_CARNAGE
};
