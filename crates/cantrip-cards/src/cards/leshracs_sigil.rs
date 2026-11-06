use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, triggers};
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a green spell" (CR 603.2).
fn when_opponent_casts_green_spell(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let Event::SpellCast { player, .. } = event else { return None };
    if !q.state().opponents(asker.controller).any(|opponent| opponent == *player) {
        return None;
    }
    let mut fired = triggers::a_player_casts_a_spell_of_color::<{ Color::Green as u8 }>(q, asker, event)?;
    fired.data[0] = u32::from(player.0);
    Some(fired)
}

/// "Whenever an opponent casts a green spell, you may pay {B}{B}. If you do, look at that player's hand and choose a card from it. The player discards that card." (CR 608.2, 701.9a, 118.1).
fn pay_b_b_if_do(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let paid = g.may_pay(item.controller, item.object, item.object, cost("{B}{B}"), &PAY_QUESTION)?;
    g.end_sentence();
    if paid {
        let player = PlayerId(item.data[0] as u8);
        let hand = g.state().zone(Zone::hand(player)).to_vec();
        g.look_at(item.controller, &hand);
        let chosen = choose::select(
            g, item.controller, item.object, Purpose::TakeFromHand, hand,
            select::cards(select::as_many_as_can(1)),
        )?;
        g.discard_together(&chosen);
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
            when_asked(EventMask::SPELL_CAST, when_opponent_casts_green_spell),
            pay_b_b_if_do,
            "Whenever an opponent casts a green spell, you may pay {B}{B}. If you do, look at that player's hand and choose a card from it. The player discards that card.",
        ),
        ability(
            cost("{B}{B}"),
            effects::return_this_to_hand,
            "{B}{B}: Return this enchantment to its owner's hand.",
        ),
    ],
    ..oracle::LESHRACS_SIGIL
};
