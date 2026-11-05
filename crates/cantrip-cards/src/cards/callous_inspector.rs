use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_oracle as oracle;

/// "When this creature dies, it deals 1 damage to you. Create a Clue token." The damage is dealt by the creature as it last
/// existed on the battlefield (CR 603.10a, 608.2h) to the ability's controller; the Clue is created, which is not investigating (CR 701.16a). That ruling is unexercised:
/// nothing in the engine watches for investigating, so no scenario can tell the two apart.
fn dies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::source_deals_one_to_controller(g, item)?;
    tokens::create(g, item.controller, &tokens::CLUE, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_DIES, dies, "When this creature dies, it deals 1 damage to you. Create a Clue token.")],
    ..oracle::CALLOUS_INSPECTOR
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
