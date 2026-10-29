use cantrip_api::*;
use cantrip_mechanics::triggers::ward;
use cantrip_oracle as oracle;

/// "Cast Mutant, Ninja, or Turtle spells from the top of your library" (CR 601.3): the card on top as the spell it would be, its
/// creature types read through the words of this permanent's text (CR 612).
fn mutant_ninja_or_turtle_spell(q: &dyn Query, asker: &Asker, card: ObjectId) -> bool {
    let words = q.words(asker.source);
    let c = q.chars(card);
    c.has_subtype(words.subtype(Subtype::Mutant)) || c.has_subtype(words.subtype(Subtype::Ninja)) || c.has_subtype(words.subtype(Subtype::Turtle))
}

/// "If you cast a creature spell this way, that creature enters with an additional +1/+1 counter on it" (CR 614.1c, 122.6): a
/// creature spell this permanent's permission let its controller cast (`Cast::granted_by`, which can only be from the top of the
/// library), as it enters; a spell cast from the top of the library by another permission is not cast "this way".
fn creature_spell_cast_this_way_enters(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.controller == asker.controller
            && q.chars(z.object).is(CardType::Creature)
            && q.history().cast_of(z.object).is_some_and(|c| c.granted_by().is_some_and(|g| g.object == asker.source)))
}

fn add_a_counter(_: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += 1;
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)")],
    // "You may look at the top card of your library any time" (CR 401.2), and "you may play lands and cast Mutant, Ninja, or Turtle
    // spells from the top of your library" (CR 305.1, 601.3).
    play_rules: &[PlayRule::LookAtLibraryTop, PlayRule::LibraryTopLands, PlayRule::LibraryTopSpells(mutant_ninja_or_turtle_spell)],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        creature_spell_cast_this_way_enters,
        add_a_counter,
        "puts an additional +1/+1 counter on it",
    )],
    ..oracle::MIKEY_DON_PARTY_PLANNERS
};
