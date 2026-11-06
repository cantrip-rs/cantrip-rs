use cantrip_api::*;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice Nicol Bolas unless you pay {U}{B}{R}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_nicol_bolas_unless_pay(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.may_pay(item.controller, item.object, item.object, cost("{U}{B}{R}"), &PAY_QUESTION)? {
        g.sacrifice(item.controller, &[item.object]);
    }
    Ok(())
}

/// "Whenever Nicol Bolas deals damage to an opponent" (CR 603.2).
fn when_nicol_bolas_deals_damage_opponent(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    if let Event::Damage { source, target: Target::Player(player), amount, .. } = event
        && *source == asker.source && *amount > 0
        && q.state().opponents(asker.controller).any(|p| p == *player)
    {
        return Fired::about(Target::Player(*player));
    }
    None
}

/// "That player discards their hand" (CR 701.9a).
fn player_discards_their_hand(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = item.subject.player() {
        g.discard_hand(player);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Pay",
    "pay {cost}?",
    "",
    "Pay {cost}",
    "Decline",
);

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOUR_UPKEEP,
            sacrifice_nicol_bolas_unless_pay,
            "At the beginning of your upkeep, sacrifice Nicol Bolas unless you pay {U}{B}{R}.",
        ),
        triggered(
            when_asked(EventMask::DAMAGE, when_nicol_bolas_deals_damage_opponent),
            player_discards_their_hand,
            "Whenever Nicol Bolas deals damage to an opponent, that player discards their hand.",
        ),
    ],
    ..oracle::NICOL_BOLAS
};
