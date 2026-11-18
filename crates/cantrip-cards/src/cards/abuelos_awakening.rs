use cantrip_api::*;
use cantrip_mechanics::targets::{self, card_in_graveyard};
use cantrip_oracle as oracle;

/// "Target artifact or non-Aura enchantment card in your graveyard" (CR 115.1, 404.1): a card there is the types it has in a
/// graveyard, so an artifact creature card is an artifact and an Aura (an enchantment with the Aura subtype, read through the
/// words of the spell's text, CR 612) is not a legal target.
fn artifact_or_non_aura_enchantment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    let Target::Object(id) = t else { return false };
    let chars = q.chars(id);
    card_in_graveyard(q, t, &[CardType::Artifact, CardType::Enchantment])
        && q.obj(id).zone == Zone::graveyard(asker.controller)
        && (chars.is(CardType::Artifact) || !chars.has_subtype(q.words(asker.source).subtype(Subtype::Aura)))
}

const TARGET: TargetSpec = target(artifact_or_non_aura_enchantment_card_in_your_graveyard);

/// "It's a 1/1 Spirit creature with flying in addition to its other types" (CR 614.12, 205.1b, 613.4b): what the card enters as, so
/// what watches creatures enter sees a 1/1 flying Spirit creature, for as long as it is that object (CR 611.2a, 400.7). The subtype is
/// read through the words of the spell (CR 612.1), as Ghost Vacuum's is.
static AS_A_FLYING_SPIRIT: EntersAs = EntersAs {
    adds_types: TypeSet::of(&[CardType::Creature]),
    subtypes: &[Subtype::Spirit],
    power_toughness: Some((1, 1)),
    keywords: Keywords::of(&[Keyword::Flying]),
    ..EntersAs::NONE
};

/// "Return target artifact or non-Aura enchantment card from your graveyard to the battlefield with X additional +1/+1 counters
/// on it. It's a 1/1 Spirit creature with flying in addition to its other types." (CR 400.3, 107.3, 122.6, 614.1c): the card
/// enters under the spell's controller's control with X counters and as `AS_A_FLYING_SPIRIT`; nothing if it is no longer a legal
/// target (CR 608.2b).
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.put_all_onto_battlefield_becoming(&[(card, item.controller)], item, &AS_A_FLYING_SPIRIT, &[(CounterKind::PlusOne, item.x as i32)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[TARGET], effect),
    ..oracle::ABUELOS_AWAKENING
};
