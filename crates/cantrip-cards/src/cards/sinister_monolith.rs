use cantrip_api::*;
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            BEGINNING_OF_YOUR_COMBAT,
            effects::each_opponent_loses_1_life_you_gain_1,
            "At the beginning of combat on your turn, each opponent loses 1 life and you gain 1 life.",
        ),
        ability(TAP.and(life(2)).and(SACRIFICE_THIS), |g, item| effects::draw(g, item, 2), "{T}, Pay 2 life, Sacrifice this artifact: Draw two cards. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    ..oracle::SINISTER_MONOLITH
};
