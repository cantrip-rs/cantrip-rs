use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// The damage ability can be activated only in the end of combat step (CR 602.5).
fn end_of_combat(q: &dyn Query, _: &Asker) -> bool {
    q.state().turn.step() == Step::EndOfCombat
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add(Mana::C), "{T}: Add {C}."),
        ability(
            TAP,
            effects::damage_first_target::<1>,
            "{T}: This land deals 1 damage to target attacking creature. Activate only during the end of combat step.",
        )
        .targets(&[targets::ATTACKING_CREATURE])
        .condition(end_of_combat),
    ],
    ..oracle::DESERT
};
