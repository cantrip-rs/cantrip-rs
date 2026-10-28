use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Tap target creature. If it's your turn, put a stun counter on it. Draw a card." (CR 701.26a, 122.1d.) Whose turn it is
/// is read as the spell resolves, so a Rapier Wit cast on an opponent's turn only taps. A target no longer legal makes the
/// whole spell do nothing, the card included (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.tap(creature);
    g.end_sentence();
    if g.state().turn.active == item.controller {
        g.add_counters(creature, CounterKind::Stun, 1);
    }
    g.end_sentence();
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[targets::CREATURE], effect), ..oracle::RAPIER_WIT };
