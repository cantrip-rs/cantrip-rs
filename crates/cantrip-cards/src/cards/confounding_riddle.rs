use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets};
use cantrip_oracle as oracle;

/// "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard" (CR 701.20e, 400.3).
static MODAL: ModalDef = modes(&[
    mode(
        |g, item| library::look_at_top_take_one_rest_to_graveyard(g, item.controller, item.object, 4),
        "Look at the top four cards of your library. Put one of them into your hand and the rest into your graveyard.",
    ),
    mode(effects::counter_first_target_unless_pays::<4>, "Counter target spell unless its controller pays {4}.").targets(&[targets::SPELL]),
]);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::CONFOUNDING_RIDDLE };
