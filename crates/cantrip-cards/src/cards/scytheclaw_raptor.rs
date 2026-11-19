use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, triggers::a_player_casts_a_spell_on_a_turn_not_theirs),
        |g, item| effects::damage_that_player(g, item, 4),
        "Whenever a player casts a spell, if it's not their turn, this creature deals 4 damage to them.",
    )],
    ..oracle::SCYTHECLAW_RAPTOR
};
