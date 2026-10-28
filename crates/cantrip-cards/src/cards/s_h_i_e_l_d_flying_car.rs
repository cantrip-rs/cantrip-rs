use cantrip_api::*;
use cantrip_mechanics::crew::crew;
use cantrip_mechanics::delayed;
use cantrip_mechanics::{targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// Exile up to one target creature you control. Return that card to the battlefield under its owner's control at the
/// beginning of the next end step (CR 603.7): a delayed ability about the card exiled, only if it is in exile.
fn exile(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    delayed::exile_target_then_return_at_next_end_step(g, item, 0, 0);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    // "Exile up to one target creature you control. Return that card to the battlefield under its owner's control at the
    // beginning of the next end step." (CR 603.7): a delayed ability about the card exiled.
    abilities: &[
        triggered(
            THIS_ENTERS,
            exile,
            "When this Vehicle enters, exile up to one target creature you control. Return that card to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[up_to_one(targets::CREATURE_YOU_CONTROL.filter)]),
        crew(1, "Crew 1 (Tap any number of creatures you control with total power 1 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return the card exiled with S.H.I.E.L.D. Flying Car to the battlefield under its owner's control.",
    )],
    ..oracle::S_H_I_E_L_D_FLYING_CAR
};
