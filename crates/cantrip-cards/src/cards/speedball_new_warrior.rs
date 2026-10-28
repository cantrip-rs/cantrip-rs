use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever a player casts a spell that targets Speedball": the cast is the event, the spell the ability is about
/// (`StackItem::subject`). Its targets are read from the stack: the cast is checked before they are recorded as targets.
fn a_player_casts_a_spell_that_targets_this(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, .. } = event else { return None };
    let item = q.state().stack.iter().filter(|i| i.kind == StackKind::Spell).find(|i| i.object == *spell)?;
    if !q.state().targets(item).iter().any(|t| t.object() == Some(me.source)) {
        return None;
    }
    Fired::about(Target::Object(*spell))
}

/// "He gets +2/+2 until end of turn. You may choose new targets for that spell." (CR 611.2a, 115.7d.) Speedball's controller
/// may retarget the spell, whoever cast it, if it is still on the stack; each slot offers its current target among the others.
fn pump_and_retarget(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::pump(g, item.object, item.object, 2, 2);
    match item.subject.object() {
        Some(spell) => g.choose_new_targets(spell, item.controller),
        None => Ok(()),
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, a_player_casts_a_spell_that_targets_this),
        pump_and_retarget,
        "Whenever a player casts a spell that targets Speedball, he gets +2/+2 until end of turn. You may choose new targets for that spell.",
    )],
    ..oracle::SPEEDBALL_NEW_WARRIOR
};
