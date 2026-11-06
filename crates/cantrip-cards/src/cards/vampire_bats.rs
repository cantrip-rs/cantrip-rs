use cantrip_api::*;
use cantrip_mechanics::{effects, history};
use cantrip_oracle as oracle;

/// "Activate no more than twice each turn" (CR 602.5b, 602.5c).
fn fewer_than_two_activations(q: &dyn Query, asker: &Asker) -> bool {
    history::this_ability_activations_this_turn(q, asker) < 2
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{B}"),
        effects::this_gets::<1, 0>,
        "{B}: This creature gets +1/+0 until end of turn. Activate no more than twice each turn.",
    ).condition(fewer_than_two_activations)],
    ..oracle::VAMPIRE_BATS
};
