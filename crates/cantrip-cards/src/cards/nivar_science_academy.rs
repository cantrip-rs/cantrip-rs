use cantrip_api::*;
use cantrip_mechanics::{effects, mana, targets, triggers};
use cantrip_oracle as oracle;

static WHEN_UNTAPPED: ModalDef = modes(&[
    mode(effects::first_target_gains_flying, "Target creature gains flying until end of turn.")
        .targets(&[targets::CREATURE]),
    mode(|g, item| {
        effects::surveil(g, item, 1)
    }, "Surveil 1."),
]);

/// "This land enters tapped unless you control two or more other lands" and "When this land enters untapped, choose one".
/// The shared entry replacement counts other lands; the trigger only fires if the land entered untapped (CR 614.1d, 603.6d).
pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped_unless_two_or_more_other_lands),
    abilities: &[
        modal_triggered(
            triggers::THIS_ENTERS_UNTAPPED,
            &WHEN_UNTAPPED,
            "When this land enters untapped, choose one — • Target creature gains flying until end of turn. • Surveil 1.",
        ),
        mana_ability(TAP, add(Mana::U), "{T}: Add {U}."),
    ],
    ..oracle::NIVAR_SCIENCE_ACADEMY
};
