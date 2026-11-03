use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Whenever a player casts a green spell, you may pay {1}. If you do, you gain 1 life." (CR 603.1).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(
                EventMask::SPELL_CAST,
                triggers::a_player_casts_a_spell_of_color::<{ Color::Green as u8 }>,
            ),
            |g, item| effects::may_pay_to_gain_life(g, item, cost("{1}")),
            "Whenever a player casts a green spell, you may pay {1}. If you do, you gain 1 life.",
        ),
    ],
    ..oracle::WOODEN_SPHERE
};
