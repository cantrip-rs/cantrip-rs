use cantrip_api::*;
use cantrip_mechanics::{choose, history};
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "As Gollum enters, choose odd or even" (CR 614.12a, 607.2f), kept on the permanent as the chosen quality. Zero is even.
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    choose::option_as_enters(g, entering, &CARD, 0..2)
}

/// "Whenever an opponent casts a spell with mana value of the chosen quality": a spell, not an ability, and one of an opponent's.
fn opponent_casts_a_matching_spell(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let odd = choose::chosen_option(q, asker, &CARD)? == 0;
    if *player == asker.controller || !choose::mana_value_of_quality(q, *spell, odd) {
        return None;
    }
    Fired::PLAIN
}

/// "Put a +1/+1 counter on Gollum": nothing to put it on once Gollum has left.
fn counter(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if targets::permanent(g, item.object) {
        g.add_counters(item.object, CounterKind::PlusOne, 1);
    }
    Ok(())
}

/// "Draw a card."
fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

static RIDDLE: ModalDef = modes(&[
    mode(counter, "Put a +1/+1 counter on Gollum."),
    mode(cantrip_mechanics::effects::each_opponent_loses_life_you_gain_life::<2>, "Each opponent loses 2 life and you gain 2 life."),
    mode(draw, "Draw a card."),
])
    .allowed(history::mode_not_chosen_before);

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    labels: choose::ODD_OR_EVEN,
    abilities: &[modal_triggered(when_asked(EventMask::SPELL_CAST, opponent_casts_a_matching_spell), &RIDDLE, "Whenever an opponent casts a spell with mana value of the chosen quality, choose one that hasn't been chosen \u{2014} put a +1/+1 counter on Gollum; or each opponent loses 2 life and you gain 2 life; or draw a card.")],
    ..oracle::GOLLUM_RIDDLE_MASTER
};
