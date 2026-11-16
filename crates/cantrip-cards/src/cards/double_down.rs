use cantrip_api::*;
use cantrip_mechanics::outlaw;
use cantrip_oracle as oracle;

/// "Whenever you cast an outlaw spell" (CR 603.2, 700.12): a spell its controller casts that has Assassin, Mercenary, Pirate, Rogue
/// and/or Warlock among its types as it stands on the stack, a kindred spell included. It is the trigger's subject, so the copy is
/// of that spell.
fn you_cast_an_outlaw_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::SpellCast { spell, player, .. } if *player == me.controller && outlaw::is_outlaw(&q.chars(*spell)) => {
            Fired::about(Target::Object(*spell))
        }
        _ => None,
    }
}

/// "Copy that spell." The text gives no new targets, so the copy keeps the spell's (CR 707.10). It is not cast, so it sets nothing
/// off (CR 707.10), and a copy of a permanent spell becomes a token as it resolves (CR 608.3f).
fn copy_that_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    g.copy_spell(spell, item.controller, false)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_an_outlaw_spell),
        copy_that_spell,
        "Whenever you cast an outlaw spell, copy that spell. (Assassins, Mercenaries, Pirates, Rogues, and Warlocks are outlaws. Copies of permanent spells become tokens.)",
    )],
    ..oracle::DOUBLE_DOWN
};
