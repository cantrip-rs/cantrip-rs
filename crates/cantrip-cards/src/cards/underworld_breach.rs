use cantrip_api::*;
use cantrip_mechanics::escape;
use cantrip_oracle as oracle;

/// "Each nonland card in your graveyard has escape. The escape cost is equal to the card's mana cost plus exile three
/// other cards from your graveyard" (CR 702.138a, 113.6, 604): for the cards in its controller's graveyard that have a
/// mana cost, cast by that controller. "Nonland card" is the card in the graveyard, which has only its normal
/// characteristics there (CR 715.4): a land card with an Adventure has no escape (Lindblum, Industrial Regency). The cost
/// is that of the face cast (`q` reads the spell as it, CR 709.3a).
fn escape_cost(q: &dyn Query, asker: &Asker, card: ObjectId, caster: PlayerId) -> Option<Cost> {
    if caster != asker.controller || q.obj(card).owner != asker.controller || q.card(q.obj(card).card).is(CardType::Land) {
        return None;
    }
    let def = q.card(q.chars(card).card);
    if def.is(CardType::Land) || !def.has_cost {
        return None;
    }
    Some(escape::cost(def.cost, 3))
}

pub static CARD: CardDef = CardDef {
    granted_ways: &[escape::granted(escape_cost)],
    abilities: &[triggered(cantrip_mechanics::triggers::EACH_END_STEP, cantrip_mechanics::effects::sacrifice_self, "At the beginning of the end step, sacrifice this enchantment.")],
    ..oracle::UNDERWORLD_BREACH
};
