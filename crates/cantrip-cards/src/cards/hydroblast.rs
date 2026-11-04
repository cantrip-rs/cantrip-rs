use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Counter target spell if it's red." (CR 608.2h, 701.6a).
fn counter_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(spell) = targets::object(g, item, 0)
        && g.chars(spell).colors.has(g.words(item.object).color(Color::Red))
    {
        g.counter(spell);
    }
    Ok(())
}

/// "Destroy target permanent if it's red." (CR 608.2h, 701.8a).
fn destroy_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(permanent) = targets::object(g, item, 0)
        && g.chars(permanent).colors.has(g.words(item.object).color(Color::Red))
    {
        g.destroy(permanent);
    }
    Ok(())
}

static MODAL: ModalDef = modes(&[
    mode(counter_red, "Counter target spell if it's red.").targets(&[targets::SPELL]),
    mode(destroy_red, "Destroy target permanent if it's red.").targets(&[targets::PERMANENT]),
]).choose(1, 1);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::HYDROBLAST };
