use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell with a single target" (CR 115.1): the spell has exactly one target, as it
/// was announced (CR 601.2c).
fn you_cast_an_instant_or_sorcery_with_a_single_target(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let chars = q.chars(*spell);
    if *player != q.controller(source) || !(chars.is(CardType::Instant) || chars.is(CardType::Sorcery)) {
        return None;
    }
    let state = q.state();
    let item = state.stack.iter().find(|i| i.kind == StackKind::Spell && i.object == *spell).or_else(|| state.left_stack(*spell))?;
    if state.targets(item).len() != 1 {
        return None;
    }
    Fired::about(Target::Object(*spell))
}

/// "You may copy it. If you do, those spells gain wither. You may choose new targets for the copy."
fn copy_it_and_both_gain_wither(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(spell) = item.subject.object() else { return Ok(()) };
    if g.ask(item.controller, RequestKind::May { source: item.object, purpose: Purpose::Ask(&COPY_IT) })? != Answer::Yes {
        return Ok(());
    }
    let wither = EffectKind::AddKeywords(Keywords::of(&[Keyword::Wither]));
    g.add_effect(item.object, spell, wither.clone(), Duration::Indefinite);
    if let Some(copy) = g.copy_spell(spell, item.controller, true)? {
        g.add_effect(item.object, copy, wither, Duration::Indefinite);
    }
    Ok(())
}

const COPY_IT: Question = Question::new("CopySpell", "copy the spell?", "Both spells gain wither, and you may choose new targets for the copy", "Copy it", "Don't");

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when(EventMask::SPELL_CAST, you_cast_an_instant_or_sorcery_with_a_single_target),
        copy_it_and_both_gain_wither,
        "Whenever you cast an instant or sorcery spell with a single target, you may copy it. If you do, those spells gain wither. You may choose new targets for the copy.",
    )],
    ..oracle::SPINEROCK_TYRANT
};
