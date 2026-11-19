use cantrip_api::*;
use cantrip_mechanics::triggers::you_attack_with_a_creature_of_subtype;
use cantrip_mechanics::{effects, history, lands, library, targets, tokens};
use cantrip_oracle as oracle;

static MODAL: ModalDef = modes(&[
    mode(tokens::create_a_treasure, "Create a Treasure token."),
    mode(effects::first_target_cant_block_this_turn, "Target creature can't block this turn.").targets(&[targets::CREATURE]),
    mode(library::exile_top_card_play_this_turn, "Exile the top card of your library. You may play it this turn."),
])
.allowed(history::mode_not_chosen_this_turn);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        when_asked(EventMask::ATTACKS, you_attack_with_a_creature_of_subtype::<{ lands::sub(Subtype::Pirate) }>),
        &MODAL,
        "Whenever a Pirate you control attacks, choose one that hasn't been chosen this turn \u{2014} Create a Treasure token; or target creature can't block this turn; or exile the top card of your library. You may play it this turn.",
    )],
    ..oracle::BREECHES_EAGER_PILLAGER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
