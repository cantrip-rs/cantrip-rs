use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// "Create four 1/1 colorless Hero creature tokens. Then put a +1/+1 counter on each creature you control." (CR 111.1, 122.1,
/// 608.2c): the four are created together (CR 101.4), then counted among the creatures that get a counter.
fn chosen(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::HERO_1_1_COLORLESS, 4)?;
    effects::plus_one_counter_on_each_creature_you_control(g, item)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], chosen), ..oracle::THE_CRYSTALS_CHOSEN };

pub static EXTRA: &[&CardDef] = &[&tokens::HERO_1_1_COLORLESS];
