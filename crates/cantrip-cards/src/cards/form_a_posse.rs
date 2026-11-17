use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create X 1/1 red Mercenary creature tokens with "{T}: Target creature you control gets +1/+0 until end of turn. Activate
/// only as a sorcery."" X is the number chosen, not the mana spent (CR 107.3a), and the tokens are created together, as one
/// event (CR 111.1).
fn create_x_mercenaries(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::MERCENARY, item.x)?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], create_x_mercenaries), ..oracle::FORM_A_POSSE };

pub static EXTRA: &[&CardDef] = &[&tokens::MERCENARY];
