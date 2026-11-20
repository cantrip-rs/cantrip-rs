use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, triggers::source_you_control_deals_noncombat_damage_to_an_opponent),
        effects::draw_that_many,
        "Whenever a source you control deals noncombat damage to an opponent, you draw that many cards.",
    )
    .damage_amount()],
    play_rules: &[PlayRule::MaximumHandSize(None)],
    ..oracle::NIV_MIZZET_VISIONARY
};
