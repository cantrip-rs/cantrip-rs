use cantrip_api::*;
use cantrip_mechanics::{affinity, history};
use cantrip_oracle as oracle;

/// "Whenever an opponent attacks with creatures" (CR 508.1, 603.2): one trigger for the declaration of an opponent's attackers,
/// whatever they attack, the ability being about that opponent (`Fired::about`) - "that opponent".
fn opponent_attacks_with_creatures(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::AttackersDeclared { player, .. } if q.state().opponents(me.controller).any(|o| o == *player) => Fired::about(Target::Player(*player)),
        _ => None,
    }
}

/// "If two or more of those creatures are attacking you and/or planeswalkers you control" (CR 603.4, 508.1): of the creatures that
/// opponent declared, those attacking this ability's controller or a planeswalker they control now. Asked as the ability
/// triggers and again as it resolves, so a creature that has left combat, or whose planeswalker has, no longer counts. The history
/// leaves out the batch being judged (as it triggers every attacker was just declared); once the declaration is in it, "those
/// creatures" are the ones it holds, and a creature put onto the battlefield attacking afterwards (CR 508.4) is not.
fn two_or_more_attack_you_or_your_planeswalkers(q: &dyn Query, asker: &Asker) -> bool {
    let declared = history::this_combat(q).iter().any(|r| matches!(r.event, Event::AttackersDeclared { .. }));
    q.state()
        .attackers
        .iter()
        .filter(|a| {
            a.target == Target::Player(asker.controller)
                || matches!(a.target, Target::Object(o) if q.controller(o) == asker.controller && q.chars(o).is(CardType::Planeswalker))
        })
        .filter(|a| !declared || history::attacked_this_combat(q, a.id))
        .count()
        >= 2
}

/// "That opponent loses 3 life and you draw a card."
fn lose_three_and_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(opponent) = item.subject {
        g.lose_life(opponent, 3);
    }
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    own_cost_changes: &[affinity::for_planeswalkers],
    abilities: &[triggered(
        when_asked(EventMask::ATTACKERS_DECLARED, opponent_attacks_with_creatures),
        lose_three_and_draw,
        "Whenever an opponent attacks with creatures, if two or more of those creatures are attacking you and/or planeswalkers you control, that opponent loses 3 life and you draw a card.",
    )
    .condition(two_or_more_attack_you_or_your_planeswalkers)],
    ..oracle::TOMIK_WIELDER_OF_LAW
};
