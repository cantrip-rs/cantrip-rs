use cantrip_api::*;
use cantrip_mechanics::effects::draw_two_each_player_loses_two;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], draw_two_each_player_loses_two), ..oracle::RISKY_SHORTCUT };
