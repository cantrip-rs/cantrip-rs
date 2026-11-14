use cantrip_api::*;
use cantrip_mechanics::{copy, triggers};
use cantrip_oracle as oracle;

/// The instant and sorcery spells `player` cast this turn before `spell` was cast (CR 601.2i), as each was cast: one that was
/// countered or has left the stack since still counts, a copy (not cast, CR 707.10) never does, and `spell` itself and any spell cast
/// after it do not. Only Thousand-Year Storm prints "each other instant and sorcery spell you've cast before it", so it lives here.
fn instants_and_sorceries_cast_before(q: &dyn Query, player: PlayerId, spell: ObjectId) -> u32 {
    q.history()
        .casts_by(player)
        .take_while(|c| c.spell != spell)
        .filter(|c| c.is(CardType::Instant) || c.is(CardType::Sorcery))
        .count() as u32
}

/// "Whenever you cast an instant or sorcery spell" (CR 603.2), counting as it triggers, as storm does: the
/// instant and sorcery spells its controller cast this turn before that one are fixed then, so a spell cast in response to this
/// ability is not "before it". The spell rides along as "it".
fn cast_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::you_cast_instant_or_sorcery(q, me, event)?;
    let spell = fired.subject.object()?;
    Some(Fired { data: [instants_and_sorceries_cast_before(q, me.controller, spell), 0], ..fired })
}

/// "Copy it for each other instant and sorcery spell you've cast before it this turn. You may choose new targets for the copies"
/// (CR 707.10, 707.10c): that many copies, none cast (so this ability does not trigger for them), made even if the spell has been
/// countered, from its last-known information if it has left the stack (CR 608.2h); none if it was the first.
fn copy_for_each_earlier(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    copy::copy_the_cast_spell_times(g, item, item.data[0])
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::SPELL_CAST, cast_instant_or_sorcery),
        copy_for_each_earlier,
        "Whenever you cast an instant or sorcery spell, copy it for each other instant and sorcery spell you've cast before it this turn. You may choose new targets for the copies.",
    )],
    ..oracle::THOUSAND_YEAR_STORM
};
