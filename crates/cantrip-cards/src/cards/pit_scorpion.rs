use cantrip_api::*;
use cantrip_mechanics::poison;
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage to a player, that player gets a poison counter. (A player with ten or more poison counters loses the game.)" (CR 603.1).
pub static CARD: CardDef = CardDef {
    abilities: &[
        poison::on_damage_to_player::<
            1,
        >(
            "Whenever this creature deals damage to a player, that player gets a poison counter.",
        ),
    ],
    ..oracle::PIT_SCORPION
};
