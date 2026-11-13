use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers::THIS_ATTACKS};
use cantrip_oracle as oracle;

fn surveil_one(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::surveil(g, item, 1)
}

static MODAL: ModalDef = modes(&[
    mode(effects::first_target_gains_until_end_of_turn::<{ Keywords::of(&[Keyword::Deathtouch]).0 }>, "Target creature you control gains deathtouch until end of turn.")
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(surveil_one, "Surveil 1."),
]);

pub static CARD: CardDef = CardDef {
    abilities: &[modal_triggered(
        THIS_ATTACKS,
        &MODAL,
        "Whenever this creature attacks, choose one — target creature you control gains deathtouch until end of turn; or surveil 1.",
    )],
    ..oracle::TAL_SHIAR_OPERATIVE
};
