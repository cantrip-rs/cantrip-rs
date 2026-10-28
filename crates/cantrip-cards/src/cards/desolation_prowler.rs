use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(life(2), effects::this_gets::<2, 2>, "Pay 2 life: This creature gets +2/+2 until end of turn. Activate only once each turn.").once_each_turn()],
    ..oracle::DESOLATION_PROWLER
};
