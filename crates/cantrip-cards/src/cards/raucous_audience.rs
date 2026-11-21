use cantrip_api::*;
use cantrip_mechanics::ferocious;
use cantrip_oracle as oracle;

/// "{T}: Add {G}. If you control a creature with power 4 or greater, add {G}{G} instead." One mana ability: the creature is looked
/// for as the ability resolves (CR 605.3b, it does not use the stack), and the pool gets {G}{G} in place of {G}, never three.
fn add_one_green_or_two_if_ferocious(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let amount = if ferocious::you_control(g, asker.controller) { 2 } else { 1 };
    g.add_mana(asker.controller, Mana::G, amount);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_by(add_one_green_or_two_if_ferocious),
        "{T}: Add {G}. If you control a creature with power 4 or greater, add {G}{G} instead.",
    )],
    ..oracle::RAUCOUS_AUDIENCE
};
