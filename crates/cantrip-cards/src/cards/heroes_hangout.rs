use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::{library, targets};
use cantrip_oracle as oracle;

/// "Patrol Night - One or two target creatures each get +1/+0 and gain first strike until end of turn."
fn patrol_night(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::targets_get_and_gain(g, item, 1, 0, Keyword::FirstStrike)
}

static MODAL: ModalDef = modes(&[
    mode(library::exile_top_two_choose_one_play_until_end_of_next_turn, "Date Night — Exile the top two cards of your library. Choose one of them. Until the end of your next turn, you may play that card."),
    mode(patrol_night, "Patrol Night — One or two target creatures each get +1/+0 and gain first strike until end of turn.")
        .targets(&[targets::CREATURE.between(1, 2)]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::HEROES_HANGOUT };
