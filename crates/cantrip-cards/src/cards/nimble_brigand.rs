use cantrip_api::*;
use cantrip_mechanics::statics::{have_cant_be_blocked, itself_while_you_committed_a_crime};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "This creature can't be blocked if you've committed a crime this turn" (CR 509.1b, 700.13, 604.1): it can't be blocked while its
/// controller has committed a crime this turn, asked again whenever characteristics are computed, so a crime after blockers
/// are declared does not undo a block (the ruling). "Whenever this creature deals combat damage to a player, draw a card" (CR 510.2).
pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself_while_you_committed_a_crime,
        have_cant_be_blocked,
        "This creature can't be blocked if you've committed a crime this turn. (Targeting opponents, anything they control, and/or cards in their graveyards is a crime.)",
    )],
    abilities: &[triggered(
        triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER,
        |g, item| effects::draw(g, item, 1),
        "Whenever this creature deals combat damage to a player, draw a card.",
    )],
    ..oracle::NIMBLE_BRIGAND
};
