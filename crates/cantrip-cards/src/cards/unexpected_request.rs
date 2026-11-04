use cantrip_api::*;
use cantrip_mechanics::attach::equipment_controlled;
use cantrip_mechanics::{choose, delayed, effects, select, targets};
use cantrip_oracle as oracle;

/// "Gain control of target creature until end of turn. Untap that creature. It gains haste until end of turn. You may attach an
/// Equipment you control to that creature. If you do, unattach it at the beginning of the next end step." (CR 701.3a, 603.7):
/// the first three sentences are Act of Treason's. The Equipment is one of those its controller controls as this resolves,
/// chosen or not; it is "attached this way" only if it really became attached (`Game::attach`: not when it could not be, CR
/// 701.3b, nor when it already was), and only then is the delayed unattaching set up, about that Equipment.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = effects::gain_control_untap_haste(g, item) else { return Ok(()) };
    let equipment = equipment_controlled(g, item.controller);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Attach, equipment, select::cards(select::up_to(1)))?;
    if let Some(&attachment) = chosen.first() {
        if g.attach(attachment, creature) {
            delayed::at_next_end_step(g, item, 0, Target::Object(attachment));
        }
    }
    Ok(())
}

/// "Unattach it" (CR 701.3d) at the beginning of the next end step: the Equipment, wherever it has been attached since;
/// `Game::unattach` does nothing for an object that has left the battlefield (CR 400.7).
fn unattach_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Object(it) = item.subject {
        g.unattach(it);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE], effect),
    delayed: &[delayed_trigger(unattach_it, "Unattach it at the beginning of the next end step.")],
    ..oracle::UNEXPECTED_REQUEST
};
