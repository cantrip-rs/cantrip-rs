//! Flurry (an ability word, CR 207.2c): "Whenever you cast your second spell each turn, [effect]." Cori-Steel Cutter and Cosmogrand Zenith print it.
//!
//! A card with flurry lists `trigger(text, effect)` among its `abilities`, or `modal_trigger(text, modes)` when the
//! effect is a choice of modes (CR 700.2b).

use cantrip_api::*;

/// "Whenever you cast your second spell each turn" (CR 601.2i, 603.2): the source's controller's, counted from the turn's start
/// (`Event::SpellCast::nth`), so the spells cast before the permanent arrived count too. The spell is the subject, for "that
/// spell". Breeches, the Blastmaker and Cori-Steel Cutter print it.
fn your_second_spell(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, nth: 2, .. } if *player == q.controller(source) => Fired::about(Target::Object(*spell)),
        _ => None,
    }
}

pub const YOU_CAST_YOUR_SECOND_SPELL: Trigger = when(EventMask::SPELL_CAST, your_second_spell);

pub const fn trigger(text: &'static str, effect: fn(&mut dyn Game, &StackItem) -> Result<(), Suspend>) -> AbilityDef {
    triggered(YOU_CAST_YOUR_SECOND_SPELL, effect, text)
}

/// The same trigger with modes, chosen as it is put on the stack (CR 603.3c, 700.2b).
pub const fn modal_trigger(modes: &'static ModalDef, text: &'static str) -> AbilityDef {
    modal_triggered(YOU_CAST_YOUR_SECOND_SPELL, modes, text)
}
