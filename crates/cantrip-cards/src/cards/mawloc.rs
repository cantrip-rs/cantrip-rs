use cantrip_api::*;
use cantrip_mechanics::ravenous;
use cantrip_mechanics::fight;
use cantrip_mechanics::replacements::EXILE_INSTEAD_OF_DYING;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    enters: Some(ravenous::enters),
    abilities: &[
        ravenous::draw_trigger("When Mawloc enters, if X is 5 or more, draw a card."),
        triggered(THIS_ENTERS, fight::this_fights_target_exiling_it_if_it_would_die, "Terror from the Deep \u{2014} When Mawloc enters, it fights up to one target creature an opponent controls. If that creature would die this turn, exile it instead.")
            .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS.optional()]),
    ],
    granted_replacements: &[EXILE_INSTEAD_OF_DYING.text("If that creature would die this turn, exile it instead.")],
    ..oracle::MAWLOC
};
