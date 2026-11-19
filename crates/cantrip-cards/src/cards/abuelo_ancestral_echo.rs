use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, triggers};
use cantrip_oracle as oracle;

/// "{1}{W}{U}: Exile another target creature or artifact you control. Return it to the battlefield under its owner's control at the
/// beginning of the next end step." (CR 603.7, 400.7): the exile and the delayed ability that returns it are one effect, so it
/// returns as a new object under its owner's control, even if Abuelo is gone by then, and a token exiled this way has ceased to
/// exist and stays gone (CR 111.7). The shared slot-0 body.
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggers::ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        ability(
            cost("{1}{W}{U}"),
            delayed::exile_target_then_return_at_next_end_step_0,
            "{1}{W}{U}: Exile another target creature or artifact you control. Return it to the battlefield under its owner's control at the beginning of the next end step.",
        )
        .targets(&[targets::ANOTHER_ARTIFACT_OR_CREATURE_YOU_CONTROL]),
    ],
    delayed: &[delayed_trigger(
        delayed::return_subject_to_battlefield,
        "Return it to the battlefield under its owner's control at the beginning of the next end step.",
    )],
    ..oracle::ABUELO_ANCESTRAL_ECHO
};
