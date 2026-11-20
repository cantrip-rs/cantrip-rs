use cantrip_api::*;
use cantrip_mechanics::{choose, effects};
use cantrip_oracle as oracle;

/// "As Talion enters, choose a number between 1 and 10" (CR 107.1, 614.12a, 607.2d): an inclusive range, kept with this text for the
/// linked trigger, and not a copiable value.
fn choose_a_number_as_it_enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    choose::number_as_enters(g, entering, 1, 10, "Choose a number between 1 and 10")
}

/// "Whenever an opponent casts a spell with mana value, power, or toughness equal to the chosen number" (CR 603.2, 601.2i, 607.2d): a
/// spell cast, not one copied (CR 707.10), by a player other than Talion's controller, read as it stands on the stack: its mana value,
/// where an {X} in its cost counts as the X chosen for it (CR 202.3e), or the power or toughness of a creature spell. A spell with
/// no power or toughness never matches through them, and the trigger fires once however many of the three match. The trigger is
/// about the player who cast it ("that player").
fn opponent_casts_a_spell_with_the_chosen_number(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let chosen = choose::chosen_number(q, asker)?;
    let chars = q.chars(*spell);
    let chosen = i32::try_from(chosen).ok()?;
    (*player != asker.controller && [Some(q.mana_value(*spell) as i32), chars.power, chars.toughness].contains(&Some(chosen)))
        .then(|| Fired::about(Target::Player(*player)))
        .flatten()
}

/// "That player loses 2 life and you draw a card." (CR 119.3, 121.1)
fn that_player_loses_2_and_you_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::that_player_loses_life::<2>(g, item)?;
    g.draw(item.controller, 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enters: Some(choose_a_number_as_it_enters),
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, opponent_casts_a_spell_with_the_chosen_number),
        that_player_loses_2_and_you_draw,
        "Whenever an opponent casts a spell with mana value, power, or toughness equal to the chosen number, that player loses 2 life and you draw a card.",
    )],
    ..oracle::TALION_THE_KINDLY_LORD
};
