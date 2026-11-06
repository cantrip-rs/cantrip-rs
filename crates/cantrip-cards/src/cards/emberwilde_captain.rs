use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "Whenever an opponent attacks you" (CR 508.1): one declaration, however many creatures attack, with at least one
/// attacking the controller directly, not a planeswalker or battle they control (CR 506.2). "That player" is the
/// attacking player.
fn opponent_attacks_you(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let me = q.controller(source);
    match event {
        Event::AttackersDeclared { player, .. }
            if q.state().attackers.iter().any(|a| a.target == Target::Player(me)) =>
        {
            Fired::about(Target::Player(*player))
        }
        _ => None,
    }
}

/// "While you're the monarch" (CR 725.1): an intervening "if" (CR 603.4), checked as it triggers and as it resolves.
fn you_are_the_monarch(q: &dyn Query, asker: &Asker) -> bool {
    q.state().player(asker.controller).monarch
}

/// Dealt by the permanent, equal to the number of cards in that player's hand as the ability resolves.
fn burn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(p) = item.subject {
        let cards = cantrip_mechanics::effects::damage_for_cards_in_hand(g, p);
        g.damage(item.object, Target::Player(p), cards);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, cantrip_mechanics::effects::become_monarch, "When this creature enters, you become the monarch."),
        triggered(when(EventMask::ATTACKERS_DECLARED, opponent_attacks_you), burn, "Whenever an opponent attacks you while you're the monarch, this creature deals damage to that player equal to the number of cards in their hand.")
            .condition(you_are_the_monarch),
    ],
    ..oracle::EMBERWILDE_CAPTAIN
};
