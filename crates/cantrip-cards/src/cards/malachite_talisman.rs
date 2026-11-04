use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::a_player_casts_a_spell_of_color::<{ Color::Green as u8 }>),
            effects::may_pay_three_to_untap_target,
            "Whenever a player casts a green spell, you may pay {3}. If you do, untap target permanent.",
        )
        .targets(&[targets::PERMANENT]),
    ],
    ..oracle::MALACHITE_TALISMAN
};
