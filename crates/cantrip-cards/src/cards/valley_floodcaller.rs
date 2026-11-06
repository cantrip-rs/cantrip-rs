use cantrip_api::*;
use cantrip_mechanics::triggers::NONCREATURE_SPELL_YOU_CAST;
use cantrip_oracle as oracle;

/// "You may cast noncreature spells as though they had flash" (CR 307.5, 702.8): its controller's, from whatever zone
/// something lets them cast the card from (the ruling: flashback), the spell judged as the face that would be cast - an
/// Adventure, a Room's door (CR 715.3a, 709.3a).
fn noncreature_you_cast(q: &dyn Query, asker: &Asker, spell: ObjectId, caster: PlayerId) -> bool {
    caster == asker.controller && !q.chars(spell).is(CardType::Creature)
}

/// "Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them." The permanents are fixed as the
/// ability resolves (CR 611.2c), and "them" is exactly those. The creature types are read through the words of this
/// card's text (CR 612).
fn pump_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let words = g.words(item.object);
    let kinds = [Subtype::Bird, Subtype::Frog, Subtype::Otter, Subtype::Rat].map(|t| words.subtype(t));
    let chosen: Vec<ObjectId> = g
        .permanents_of(item.controller)
        .into_iter()
        .filter(|&id| {
            let chars = g.chars(id);
            kinds.iter().any(|&t| chars.has_subtype(t))
        })
        .collect();
    instruction(g, |g| {
        for &id in &chosen {
            cantrip_mechanics::pump(g, item.object, id, 1, 1);
        }
    });
    g.end_sentence();
    g.untap_together(&chosen);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        NONCREATURE_SPELL_YOU_CAST,
        pump_untap,
        "Whenever you cast a noncreature spell, Birds, Frogs, Otters, and Rats you control get +1/+1 until end of turn. Untap them.",
    )],
    flash_rules: &[noncreature_you_cast],
    ..oracle::VALLEY_FLOODCALLER
};
