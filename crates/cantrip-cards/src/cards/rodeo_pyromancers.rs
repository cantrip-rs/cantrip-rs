use cantrip_api::*;
use cantrip_mechanics::{cost_reduction, triggers};
use cantrip_oracle as oracle;

/// "Whenever you cast your first spell each turn" (CR 603.2, 601.2i): the first spell its controller has cast this turn, one cast
/// before this came to the battlefield included (so never the turn it was itself cast: it was the first spell, and was on the stack).
fn first_spell_you_cast(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    triggers::you_cast_a_spell(q, me, event)?;
    let Event::SpellCast { spell, player, .. } = event else { return None };
    if cost_reduction::first_spell_you_cast_this_turn(q, *spell, *player, |_| true) { Fired::PLAIN } else { None }
}

/// "Add {R}{R}": the effect of a triggered ability, which uses the stack (it is not a mana ability, CR 605.1b).
fn add_rr(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_mana(item.controller, Mana::R, 2);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(when_asked(EventMask::SPELL_CAST, first_spell_you_cast), add_rr, "Whenever you cast your first spell each turn, add {R}{R}.")],
    ..oracle::RODEO_PYROMANCERS
};
