use cantrip_api::*;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("Draw", "pay {cost} to draw a card?", "", "Pay {cost}", "Don't pay");

/// "Whenever you cast a spell during an opponent's turn" (CR 601.2i, 603.2): a spell cast by this permanent's controller while
/// the active player is another player.
fn cast_a_spell_during_an_opponents_turn(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { player, .. } if *player == me.controller && q.state().turn.active != me.controller => Fired::PLAIN,
        _ => None,
    }
}

/// "You may pay 1 life. If you do, draw a card." Only a player with the life is asked (CR 119.4).
fn pay_to_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, life(1), &ASK)? {
        g.draw(item.controller, 1);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, cast_a_spell_during_an_opponents_turn),
        pay_to_draw,
        "Whenever you cast a spell during an opponent's turn, you may pay 1 life. If you do, draw a card.",
    )],
    ..oracle::VORACIOUS_TOME_SKIMMER
};
