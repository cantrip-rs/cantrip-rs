use cantrip_api::*;
use cantrip_mechanics::tokens;
use cantrip_oracle as oracle;

/// "Create X 2/2 colorless Wizard Soldier creature tokens named Cadet, where X is the amount of life you gained this turn."
/// X is the total life gained this turn, counted as the spell resolves, without subtracting life lost (CR 119.3, ruling
/// 2025-07-25); the tokens are created together (CR 111.1, 101.4).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let x = g.history().life_gained(item.controller);
    tokens::create(g, item.controller, &tokens::CADET, u32::try_from(x).expect("token quantity exceeds native object domain"))?;
    Ok(())
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::GERMINATE_RECRUITS };

pub static EXTRA: &[&CardDef] = &[&tokens::CADET];
