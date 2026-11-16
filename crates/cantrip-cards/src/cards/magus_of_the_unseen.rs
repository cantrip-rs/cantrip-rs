use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{1}{U}").and(TAP),
        effects::untap_and_borrow_tapping_it_when_lost,
        "{1}{U}, {T}: Untap target artifact an opponent controls and gain control of it until end of turn. It gains haste until end of turn. When you lose control of the artifact, tap it.",
    ).targets(&[targets::ARTIFACT_AN_OPPONENT_CONTROLS])],
    delayed: &[triggered(
        triggers::YOU_LOSE_CONTROL_OF_THIS,
        effects::tap_subject,
        "When you lose control of the artifact, tap it.",
    )],
    ..oracle::MAGUS_OF_THE_UNSEEN
};
