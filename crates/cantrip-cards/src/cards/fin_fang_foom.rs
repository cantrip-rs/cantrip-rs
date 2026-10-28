use cantrip_api::*;
use cantrip_mechanics::{copy, effects, targets};
use cantrip_oracle as oracle;

/// "Whenever you cast an instant or sorcery spell that targets an artifact or land" (CR 603.2, 601.2i, 601.2c): a spell its
/// controller just cast, as it stands on the stack, with a target that is an artifact or a land on the battlefield. One trigger
/// for the spell however many such targets it has. A copy is not cast (CR 707.10), so it sets nothing off. The spell is the
/// trigger's subject, so the copy is of that spell.
fn you_cast_instant_or_sorcery_targeting_an_artifact_or_land(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::SpellCast { spell, player, .. } = event else { return None };
    let chars = q.chars(*spell);
    if *player != me.controller || !(chars.is(CardType::Instant) || chars.is(CardType::Sorcery)) {
        return None;
    }
    let item = q.state().stack.iter().find(|i| i.kind == StackKind::Spell && i.object == *spell)?;
    let hits = q.state().targets(item).iter().filter_map(|t| t.object()).any(|id| {
        targets::permanent(q, id) && {
            let target = q.chars(id);
            target.is(CardType::Artifact) || target.is(CardType::Land)
        }
    });
    if hits { Fired::about(Target::Object(*spell)) } else { None }
}

/// "Copy that spell. You may choose new targets for the copy. Put two +1/+1 counters on Fin Fang Foom." Two sentences, the counters
/// put on whether or not a copy was made (the spell may have left the stack).
fn copy_and_grow(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    copy::copy_the_cast_spell(g, item)?;
    g.end_sentence();
    effects::two_plus_one_counters_on_this(g, item)
}

/// Flying is a core keyword.
pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, you_cast_instant_or_sorcery_targeting_an_artifact_or_land),
        copy_and_grow,
        "Whenever you cast an instant or sorcery spell that targets an artifact or land, copy that spell. You may choose new targets for the copy. Put two +1/+1 counters on Fin Fang Foom.",
    )],
    ..oracle::FIN_FANG_FOOM
};
