use cantrip_api::*;
use cantrip_mechanics::{fight::targets_bite, targets};
use cantrip_oracle as oracle;

/// "Target planeswalker you control" (CR 115.1): a planeswalker on the battlefield under the caster's control.
fn planeswalker_you_control(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if q.controller(id) == asker.controller && (targets::PERMANENT.filter)(q, asker, t) && q.chars(id).is(CardType::Planeswalker))
}

/// "Target creature you control deals damage equal to its power to target creature or planeswalker an opponent controls."
/// (CR 120.3, 608.2b, 609.7; `fight::bite`.) If either target is illegal as it resolves, no damage is dealt.
fn power_bite(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    targets_bite(g, item, 0, 1);
    Ok(())
}

/// "Target planeswalker you control deals damage equal to its loyalty to target creature or planeswalker an opponent
/// controls." (CR 120.3, 306.5b, 609.7.) One-sided, the planeswalker the source. Its loyalty is the loyalty counters on it as
/// the spell resolves (CR 608.2c); if either target is illegal then no damage is dealt (CR 608.2b). A planeswalker with no
/// loyalty counters is already in the graveyard (CR 704.5i), so it is no legal target and the amount is never zero.
fn loyalty_bite(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let (Some(walker), Some(victim)) = (targets::object(g, item, 0), targets::object(g, item, 1)) {
        let loyalty = g.obj(walker).counters(CounterKind::Loyalty).max(0);
        g.damage(walker, Target::Object(victim), loyalty as u16);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(power_bite, "Target creature you control deals damage equal to its power to target creature or planeswalker an opponent controls.").targets(&[
        targets::CREATURE_YOU_CONTROL.labelled("Creature you control to deal damage equal to its power"),
        targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS.labelled("Creature or planeswalker an opponent controls to be dealt that damage"),
    ]),
    mode(loyalty_bite, "Target planeswalker you control deals damage equal to its loyalty to target creature or planeswalker an opponent controls.").targets(&[
        target(planeswalker_you_control).labelled("Planeswalker you control to deal damage equal to its loyalty"),
        targets::CREATURE_OR_PLANESWALKER_AN_OPPONENT_CONTROLS.labelled("Creature or planeswalker an opponent controls to be dealt that damage"),
    ]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::COMPEL_BRUTALITY };
