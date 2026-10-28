use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this creature dies, create a 2/1 black Villain creature token with menace." The token is made for the ability's
/// controller, who is the creature's controller as it died (CR 603.3a, 603.10a), not its owner.
fn villain(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::VILLAIN_2_1_BLACK_MENACE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, villain, "When this creature dies, create a 2/1 black Villain creature token with menace.")],
    ..oracle::AGENTS_OF_HYDRA
};

pub static EXTRA: &[&CardDef] = &[&tokens::VILLAIN_2_1_BLACK_MENACE];
