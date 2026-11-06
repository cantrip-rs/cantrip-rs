use cantrip_api::*;
use cantrip_oracle as oracle;

/// The source adds black mana, then deals 1 damage to its controller (CR 605.3b, 120.3).
fn add_black_and_hurt(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    g.add_mana(asker.controller, Mana::B, 1);
    g.damage(asker.source, Target::Player(asker.controller), 1);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::B).by(add_black_and_hurt),
        "{T}: Add {B}. This creature deals 1 damage to you.",
    )],
    ..oracle::ELVES_OF_DEEP_SHADOW
};
