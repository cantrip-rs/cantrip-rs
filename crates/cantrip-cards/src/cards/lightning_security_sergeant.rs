use cantrip_api::*;
use cantrip_mechanics::library;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT;
use cantrip_oracle as oracle;

/// "Whenever Lightning deals combat damage to a player, exile the top card of your library. You may play that card for as long as you
/// control Lightning." (CR 510.2, 611.2b, 305.1): the permission ends the first time Lightning's controller doesn't control it - it
/// leaves, phases out or changes controller - and a new Lightning is a new object (the ruling, CR 400.7).
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        DEALS_COMBAT_DAMAGE_TO_PLAYER_SUBJECT,
        library::exile_top_card_play_while_you_control,
        "Whenever Lightning deals combat damage to a player, exile the top card of your library. You may play that card for as long as you control Lightning.",
    )],
    ..oracle::LIGHTNING_SECURITY_SERGEANT
};
