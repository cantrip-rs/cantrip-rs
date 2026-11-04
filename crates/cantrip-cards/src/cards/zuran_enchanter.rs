use cantrip_api::*;
use cantrip_mechanics::{effects, targets, your_turn};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{B}").and(TAP),
        effects::target_player_discards::<1>,
        "{2}{B}, {T}: Target player discards a card. Activate only during your turn.",
    )
    .targets(&[targets::PLAYER])
    .condition(your_turn)],
    ..oracle::ZURAN_ENCHANTER
};
