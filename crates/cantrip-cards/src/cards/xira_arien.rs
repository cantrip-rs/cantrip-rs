use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}{R}{G}").and(TAP),
        |g, item| Ok(effects::target_player_draws(g, item, 0, 1)),
        "{B}{R}{G}, {T}: Target player draws a card.",
    )
    .targets(&[targets::PLAYER])],
    ..oracle::XIRA_ARIEN
};
