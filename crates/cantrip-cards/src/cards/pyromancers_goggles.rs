use cantrip_api::*;
use cantrip_mechanics::copy;
use cantrip_oracle as oracle;

/// "When that mana is spent to cast a red instant or sorcery spell" (CR 603.7a, 106.3): a delayed ability the mana ability
/// creates, riding the mana it produced (`Adds::when_spent`), so it triggers once for each mana of it spent - wherever the
/// Goggles have gone by then - and the core counts which mana was. The spell is a red instant or sorcery as it is on the
/// stack, and "red" is read through the Goggles' text (CR 612.1: Sleight of Mind changes the word). The spell rides along
/// as the trigger's subject ("that spell").
fn spent_on_a_red_instant_or_sorcery(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::SpellCast { spell, .. }
            if (q.chars(spell).is(CardType::Instant) || q.chars(spell).is(CardType::Sorcery))
                && q.chars(spell).colors.has(q.words(me.source).color(Color::Red)) =>
        {
            Fired::about(Target::Object(spell))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add_each(&[Mana::R]).when_spent(0),
        "{T}: Add {R}. When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.",
    )],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, spent_on_a_red_instant_or_sorcery),
        copy::copy_the_cast_spell,
        "When that mana is spent to cast a red instant or sorcery spell, copy that spell and you may choose new targets for the copy.",
    )],
    ..oracle::PYROMANCERS_GOGGLES
};
