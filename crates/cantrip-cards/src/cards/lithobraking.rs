use cantrip_api::*;
use cantrip_mechanics::{effects, select, tokens};
use cantrip_oracle as oracle;

/// "Create a Lander token. Then you may sacrifice an artifact. When you do, Lithobraking deals 2 damage to each creature." The
/// token is made first (CR 701.7a), so it is among the artifacts that can be sacrificed; the sacrifice is a choice made as the
/// spell resolves (CR 608.2d, 701.21a), and the reflexive ability (CR 603.12) exists only if an artifact was sacrificed.
fn lander_then_sacrifice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_lander(g, item.controller)?;
    effects::may_sacrifice_when_you_do(g, item, select::artifact_cards(select::up_to(1)))
}

/// "Lithobraking deals 2 damage to each creature": the spell as it last existed (CR 608.2h), every creature at once.
fn damage_each_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::damage_each_creature(g, item, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], lander_then_sacrifice),
    delayed: &[delayed_trigger(damage_each_creature, "When you do, Lithobraking deals 2 damage to each creature.")],
    ..oracle::LITHOBRAKING
};

pub static EXTRA: &[&CardDef] = &[&tokens::LANDER];
