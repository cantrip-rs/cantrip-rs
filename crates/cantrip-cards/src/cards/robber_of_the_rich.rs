use cantrip_api::*;
use cantrip_mechanics::cards_in_hand as hand_size;
use cantrip_mechanics::library::exile_top_may_play;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, if defending player has more cards in hand than you" (CR 508.3a, 603.4): checked
/// as it triggers, and the defending player is carried to the resolution.
fn attacks_a_richer_player(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, defending, .. } if *attacker == source => {
            let defending = *defending;
            if hand_size(q, defending) > hand_size(q, q.controller(source)) {
                Some(Fired { subject: Target::Player(defending), data: [0; 2] })
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Exile the top card of their library; during any turn you attacked with a Rogue, you may cast that card, spending mana
/// as though it were mana of any type (CR 609.4b). The intervening "if" holds again as the ability resolves.
fn plunder(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(defending) = item.subject else { return Ok(()) };
    if hand_size(g, defending) <= hand_size(g, item.controller) {
        return Ok(());
    }
    let grant = PlayGrant { any_type: true, when: Some(PlayCondition::AttackedWithThisTurn(g.words(item.object).subtype(Subtype::Rogue))), ..PlayGrant::CAST };
    exile_top_may_play(g, defending, 1, item.controller, grant);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when(EventMask::ATTACKS, attacks_a_richer_player), plunder, "Whenever this creature attacks, if defending player has more cards in hand than you, exile the top card of their library. During any turn you attacked with a Rogue, you may cast that card and you may spend mana as though it were mana of any color to cast that spell.")],
    ..oracle::ROBBER_OF_THE_RICH
};
