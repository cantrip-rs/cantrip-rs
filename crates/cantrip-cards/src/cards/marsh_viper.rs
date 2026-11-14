use cantrip_api::*;
use cantrip_mechanics::poison;
use cantrip_oracle as oracle;

/// "Whenever this creature deals damage to a player, that player gets two poison counters." (CR 603.1, 608.2c, 122.1).
pub static CARD: CardDef = CardDef {
    abilities: &[poison::on_damage_to_player::<2>(
        "Whenever this creature deals damage to a player, that player gets two poison counters.",
    )],
    ..oracle::MARSH_VIPER
};
