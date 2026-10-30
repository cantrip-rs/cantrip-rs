use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As Nyota Uhura enters, choose a number between 0 and 10" (CR 107.1, 614.12a, 607.2d). It is an inclusive range,
/// kept with this text for the linked trigger, and not a copiable value.
fn hailing_frequencies(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    choose::number_as_enters(g, entering, 0, 10, "Hailing Frequencies")
}

/// "Whenever an opponent casts a spell with mana value equal to the chosen number" (CR 603.2, 607.2d). Compare the
/// spell's mana value as cast; this is equality, and copies are not cast (CR 707.10).
fn opponent_casts_chosen_value(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let chosen = q.state().chosen_number(q.linked_to(asker))?;
    (*player != q.controller(asker.source) && q.mana_value(*spell) == chosen).then_some(Fired::default())
}

fn draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(hailing_frequencies),
    abilities: &[triggered(when_asked(EventMask::SPELL_CAST, opponent_casts_chosen_value), draw,
        "Open a Channel — Whenever an opponent casts a spell with mana value equal to the chosen number, you draw a card.")],
    ..oracle::NYOTA_UHURA_COMMUNICATIONS
};
