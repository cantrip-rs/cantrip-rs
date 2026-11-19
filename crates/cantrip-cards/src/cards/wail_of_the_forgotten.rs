use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

fn look_at_top_three(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 3)
}

static MODAL: ModalDef = modes(&[
    mode(effects::return_first_target_to_hand, "Return target nonland permanent to its owner's hand.")
        .targets(&[targets::NONLAND_PERMANENT]),
    mode(effects::target_player_discards::<1>, "Target opponent discards a card.")
        .targets(&[targets::OPPONENT]),
    mode(look_at_top_three, "Look at the top three cards of your library. Put one of them into your hand and the rest into your graveyard."),
])
    .up_to_if(targets::you_have_permanent_cards_in_graveyard::<8>, 3);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::WAIL_OF_THE_FORGOTTEN };
