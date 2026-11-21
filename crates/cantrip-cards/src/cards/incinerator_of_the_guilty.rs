use cantrip_api::*;
use cantrip_mechanics::{collect_evidence::may_collect_evidence_x, effects, triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT};
use cantrip_oracle as oracle;

const COLLECT_EVIDENCE_X: Question = Question::new(
    "CollectEvidenceX",
    "collect evidence X?",
    "When you do, {subject} deals X damage to each creature and planeswalker that player controls",
    "Collect evidence",
    "Don't",
)
.no_default();

/// "You may collect evidence X" (CR 701.59a, 107.3f): X is chosen as the ability resolves, as the payment is made -
/// exiling cards with total mana value X or greater from your graveyard, none at all for X = 0, which is still collecting
/// evidence (the ruling). Collecting is doing it, and "when you do" (CR 603.12) triggers the reflexive ability carrying X, the
/// damaged player its subject. Declining, or a graveyard that cannot make the X chosen (CR 701.59b), does not.
fn collect_evidence_x(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(x) = may_collect_evidence_x(g, item, &COLLECT_EVIDENCE_X)? {
        let this = g.card_id(&CARD);
        g.reflexive_trigger_counted(item.object, item.controller, AbilityRef::delayed(this, 0), item.subject, x);
    }
    Ok(())
}

/// "This creature deals X damage to each creature and planeswalker that player controls" (CR 120.3, 608.2c): one damage event
/// for each such permanent as the ability resolves, all dealt together; damage to a planeswalker removes loyalty (CR 306.8).
fn deal_x_to_each_of_theirs(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(victim) = item.subject else { return Ok(()) };
    effects::damage_each_creature_and_planeswalker_where(g, item, item.x, |q, id| q.controller(id) == victim);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_EVENT,
        collect_evidence_x,
        "Whenever this creature deals combat damage to a player, you may collect evidence X. When you do, this creature deals X damage to each creature and planeswalker that player controls.",
    )],
    delayed: &[delayed_trigger(deal_x_to_each_of_theirs, "When you do, this creature deals X damage to each creature and planeswalker that player controls.")],
    ..oracle::INCINERATOR_OF_THE_GUILTY
};
