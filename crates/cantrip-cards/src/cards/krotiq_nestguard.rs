use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{2}{G}"),
        effects::this_can_attack_as_though_it_didnt_have_defender,
        "{2}{G}: This creature can attack this turn as though it didn't have defender.",
    )],
    ..oracle::KROTIQ_NESTGUARD
};
