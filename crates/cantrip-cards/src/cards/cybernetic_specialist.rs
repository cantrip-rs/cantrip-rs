use cantrip_api::*;
use cantrip_mechanics::{effects, library, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// "When this creature enters, surveil 1." The surveil action looks at the top card and puts it on top or into the
/// controller's graveyard (CR 603.6a, 701.25a).
/// "{1}, {T}: Put target card from your graveyard on the bottom of your library." The target moves on resolution only if
/// it remains legal (CR 608.2b).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            THIS_ENTERS,
            |g, item| effects::surveil(g, item, 1),
            "When this creature enters, surveil 1.",
        ),
        ability(
            cost("{1}, {T}"),
            library::put_first_target_on_the_bottom,
            "{1}, {T}: Put target card from your graveyard on the bottom of your library.",
        )
        .targets(&[targets::CARD_IN_YOUR_GRAVEYARD]),
    ],
    ..oracle::CYBERNETIC_SPECIALIST
};


