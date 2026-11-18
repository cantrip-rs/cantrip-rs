use cantrip_api::*;
use cantrip_mechanics::{effects, tokens};
use cantrip_oracle as oracle;

/// Fuss: "Put a +1/+1 counter on each attacking creature you control." The creatures attacking as it resolves, all at once.
fn fuss(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let attackers: Vec<ObjectId> =
        effects::creatures_on_the_battlefield(g).into_iter().filter(|&id| g.is_attacking(id) && g.controller(id) == item.controller).collect();
    effects::counters_on_each(g, &attackers, CounterKind::PlusOne, 1);
    Ok(())
}

/// Bother: "Create three 1/1 colorless Thopter artifact creature tokens with flying. Surveil 2."
fn bother(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &tokens::THOPTER, 3)?;
    effects::surveil(g, item, 2)
}

pub static FUSS: CardDef = CardDef { spell: spell(&[], fuss), ..oracle::FUSS };

pub static BOTHER: CardDef = CardDef { spell: spell(&[], bother), ..oracle::BOTHER };

pub static CARD: CardDef = CardDef { split: Some((&FUSS, &BOTHER)), ..oracle::FUSS_BOTHER };

pub static EXTRA: &[&CardDef] = &[&FUSS, &BOTHER, &tokens::THOPTER];
