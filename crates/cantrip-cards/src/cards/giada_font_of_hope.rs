use cantrip_api::*;
use cantrip_oracle as oracle;

/// How many Angels `controller` already controls on the battlefield (CR 614.12): "Angel" is a word of this card's text (CR 612).
/// An Angel entering at the same time is not on the battlefield yet, so it is not counted.
fn angels_already_controlled(q: &dyn Query, asker: &Asker) -> i32 {
    let angel = q.words(asker.source).subtype(Subtype::Angel);
    q.state().battlefield().iter().filter(|&&id| q.controller(id) == asker.controller && q.chars(id).has_subtype(angel)).count() as i32
}

/// "Each other Angel you control enters with an additional +1/+1 counter on it for each Angel you already control" (CR 614.1c,
/// 122.6): another permanent about to enter the battlefield under this permanent's controller that is an Angel as it would exist
/// there (CR 614.12), card or token alike. Giada herself is not "other": she is not on the battlefield yet when she enters, so this static ability does not
/// apply to her (CR 614.12). With no Angel to count there is nothing to add.
fn another_angel_enters_under_you(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let angel = q.words(asker.source).subtype(Subtype::Angel);
    matches!(proposal, Proposal::ZoneChange(z)
        if z.to.is(ZoneKind::Battlefield)
            && z.controller == asker.controller
            && q.chars(z.object).has_subtype(angel))
        && angels_already_controlled(q, asker) > 0
}

fn add_a_counter_for_each_angel(q: &dyn Query, asker: &Asker, proposal: &mut Proposal) {
    let more = angels_already_controlled(q, asker);
    if let Proposal::ZoneChange(z) = proposal {
        z.counters[CounterKind::PlusOne as usize] += more as i32;
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[mana_ability(
        TAP,
        add(Mana::W).only_for(ManaRestriction::Subtypes(SubtypeSpells::of(Subtype::Angel))),
        "{T}: Add {W}. Spend this mana only to cast an Angel spell.",
    )],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        another_angel_enters_under_you,
        add_a_counter_for_each_angel,
        "puts an additional +1/+1 counter on it for each Angel you already control",
    )],
    ..oracle::GIADA_FONT_OF_HOPE
};
