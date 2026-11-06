use cantrip_api::*;
use cantrip_mechanics::graveyards;
use cantrip_mechanics::statics::{grant_activated_abilities_from, itself};
use cantrip_oracle as oracle;

/// "As long as this creature is on the battlefield, it has all activated abilities of all creature cards in all
/// graveyards." A static ability of the Ooze about itself (CR 604.1, 613.1f, layer 6): the activated abilities - mana
/// abilities and activated keywords among them, no triggered or static ability (the rulings) - of every card in every
/// graveyard that is a creature card there, as the graveyard has it now (CR 611.3a), in seat and graveyard order. Each is
/// the Ooze's own as written, so it is their source and "this creature" means it (the ruling: Cudgel Troll's
/// regeneration regenerates the Ooze). Opalescence makes permanents creatures, not cards in graveyards, so an enchantment
/// card there gives nothing. A later "loses all abilities" (Humility, Dress Down) takes this ability and with it all it
/// gave (CR 613.8a).
fn graveyard_abilities(view: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    let s = view.state();
    let cards: Vec<_> = graveyards::cards_in_graveyards(s)
        .map(|id| (id, view.chars(id)))
        .filter(|(_, card)| card.is(CardType::Creature))
        .map(|(id, card)| (id, card.card))
        .collect();
    grant_activated_abilities_from(view, &cards, chars);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself,
        graveyard_abilities,
        "As long as this creature is on the battlefield, it has all activated abilities of all creature cards in all graveyards.",
    )],
    ..oracle::NECROTIC_OOZE
};
