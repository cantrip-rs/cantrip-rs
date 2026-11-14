use cantrip_api::*;
use cantrip_mechanics::{effects, lands};
use cantrip_oracle as oracle;

/// "If that mana is spent on a Dragon creature spell" (CR 603.7a, 106.3): a delayed ability the mana ability creates, riding
/// the mana it produced (`Adds::when_spent`), so it triggers for a spell cast with that mana. The spell is a Dragon and a
/// creature as it is on the stack (CR 601.2i), the type read through this card's words (CR 612.2): an instant or sorcery that
/// makes Dragon tokens is not a creature spell. The spell rides along as the subject, which gains haste for the permanent it
/// becomes.
fn spent_on_a_dragon_creature_spell(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match *event {
        Event::SpellCast { spell, .. }
            if q.chars(spell).is(CardType::Creature) && q.chars(spell).has_subtype(lands::word(q, me.source, lands::sub(Subtype::Dragon))) =>
        {
            Fired::about(Target::Object(spell))
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::R).when_spent(0),
        "{T}: Add {R}. If that mana is spent on a Dragon creature spell, it gains haste until end of turn.",
    )],
    delayed: &[triggered(
        when_asked(EventMask::SPELL_CAST, spent_on_a_dragon_creature_spell),
        effects::subject_gains_haste_until_end_of_turn,
        "If that mana is spent on a Dragon creature spell, it gains haste until end of turn.",
    )],
    ..oracle::CARNELIAN_ORB_OF_DRAGONKIND
};
