use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever an opponent casts a spell" (CR 603.2, 601.2i): a spell cast, not one copied (CR 707.10), by an opponent of the
/// ability's controller. The trigger is about the player who cast it ("that player", `StackItem::subject`).
fn opponent_casts_a_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { player, .. } = event else { return None };
    if q.state().opponents(me.controller).any(|o| o == *player) { Fired::about(Target::Player(*player)) } else { None }
}

const DISCARD: Question = Question::new("DiscardInsteadOfLosingLife", "discard a card instead of losing 5 life?", "Otherwise you lose 5 life", "Discard a card", "Lose 5 life");

/// "That player loses 5 life unless they discard a card" (CR 608.2d, 701.9, 119.3): the player who cast the spell chooses as the
/// ability resolves, before the spell does (the rulings); with no card in hand, or when this ability may not make them discard (CR 101.2: Tamiyo, Collector of Tales), they cannot discard, so they lose the life.
fn lose_5_life_unless_discard(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    let (mut discards, mut losses) = (Vec::new(), Vec::new());
    effects::choose_to_discard_a_card_or_lose_life(g, item.controller, player, item.object, &DISCARD, 5, &mut discards, &mut losses)?;
    g.together(Together { discards: &discards, life: &losses, ..Default::default() });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_spell),
        lose_5_life_unless_discard,
        "Whenever an opponent casts a spell, that player loses 5 life unless they discard a card.",
    )],
    ..oracle::PAINFUL_QUANDARY
};
