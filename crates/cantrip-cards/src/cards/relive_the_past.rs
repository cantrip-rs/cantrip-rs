use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "They are 5/5 Elemental creatures in addition to their other types": what each returns as (`EntersAs`), from the moment it enters
/// (CR 614.12, 205.1b, 613.1d, 613.4b), so what watches creatures enter sees them. Each keeps its card types and its subtypes, and is
/// a creature, an Elemental and 5/5.
static AS_A_FIVE_FIVE_ELEMENTAL: EntersAs = EntersAs {
    adds_types: TypeSet::of(&[CardType::Creature]),
    subtypes: &[Subtype::Elemental],
    power_toughness: Some((5, 5)),
    ..EntersAs::NONE
};

/// "Non-Aura enchantment card from your graveyard" (CR 115.1, 400.7, 205.3): a card there is the types and subtypes it has in a
/// graveyard.
fn non_aura_enchantment_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Enchantment])
        && matches!(t, Target::Object(id) if !q.chars(id).has_subtype(Subtype::Aura))
}

/// "Return up to one target artifact card, up to one target land card, and up to one target non-Aura enchantment card from your
/// graveyard to the battlefield": the ones still legal (CR 608.2b) return together, as one event (CR 110.2a, 608.2c). A card chosen for
/// two of the targets (an artifact land) returns once.
fn return_them_as_elementals(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut cards: Vec<(ObjectId, PlayerId)> = Vec::new();
    for card in targets::legal_objects(g, item) {
        if !cards.iter().any(|&(c, _)| c == card) {
            cards.push((card, item.controller));
        }
    }
    if !cards.is_empty() {
        g.put_all_onto_battlefield_becoming(&cards, item, &AS_A_FIVE_FIVE_ELEMENTAL, &[])?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[
            targets::ARTIFACT_CARD_IN_YOUR_GRAVEYARD.optional(),
            targets::LAND_CARD_IN_YOUR_GRAVEYARD.optional(),
            target(non_aura_enchantment_card_in_your_graveyard).optional(),
        ],
        return_them_as_elementals,
    ),
    ..oracle::RELIVE_THE_PAST
};
