use cantrip_api::*;
use cantrip_mechanics::effects::this_gains_flying;
use cantrip_mechanics::history::another_creature_entered_under_your_control_this_turn;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks, if another creature entered the battlefield under your control this turn, this creature gains
/// flying until end of turn" (CR 603.4, 603.6a, 702.9b): the intervening "if" is asked as it attacks and again as the ability
/// resolves, of the history's `Event::Entered`, so a creature that has since died still counts. The trigger resolves before blockers
/// are declared, so the flying matters to how it is blocked (CR 509.1b).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        THIS_ATTACKS,
        this_gains_flying,
        "Whenever this creature attacks, if another creature entered the battlefield under your control this turn, this creature gains flying until end of turn.",
    )
    .condition(another_creature_entered_under_your_control_this_turn)],
    ..oracle::WATERSPOUT_WARDEN
};
