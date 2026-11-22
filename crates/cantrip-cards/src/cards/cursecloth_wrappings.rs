use cantrip_api::*;
use cantrip_mechanics::statics::{plus_pt, your_of_subtype};
use cantrip_mechanics::{embalm, lands, targets};
use cantrip_oracle as oracle;

/// "{T}: Target creature card in your graveyard gains embalm until end of turn" (CR 702.128a, 611.2a): the target gets the
/// embalm ability in `delayed`, for the turn, as any object is given an ability (CR 613.1f). A card that has left the graveyard
/// by the time this resolves is not given it.
fn target_card_gains_embalm(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.target_legal(item, 0)
        && let Target::Object(card) = item.target(0)
    {
        g.add_effect(item.object, card, EffectKind::AddAbility(AbilityRef::delayed(item.ability.card, 0)), Duration::EndOfTurn);
    }
    Ok(())
}

/// "The embalm cost is equal to its mana cost" (CR 118.7, 601.2f, 602.2b): the embalm ability's own cost change, over an
/// embalm cost of nothing, making its mana that of the card that has it, read as the cost is determined - hybrid, Phyrexian
/// and snow symbols kept, an {X} counting as 0 in a graveyard (CR 107.3g).
fn its_mana_cost(q: &dyn Query, asker: &Asker, _: Costed, _: &[Target]) -> CostDelta {
    CostDelta::more_mana(q.mana_cost(asker.source))
}

/// A card with no mana cost (Dryad Arbor) has an embalm cost that can't be paid, since it is based on that mana cost
/// (CR 118.6); no change makes it payable (CR 118.6a), so the embalm can't be activated. A cost of {0} (Memnite) is a
/// cost, paid with no mana.
fn has_a_mana_cost(q: &dyn Query, asker: &Asker) -> bool {
    q.card(q.chars(asker.source).card).has_cost
}

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        target_card_gains_embalm,
        "{T}: Target creature card in your graveyard gains embalm until end of turn. The embalm cost is equal to its mana cost.",
    )
    .targets(&[targets::CREATURE_CARD_IN_YOUR_GRAVEYARD])],
    // The ability it gives: embalm costing the mana cost of the card that has it (CR 702.128a, 118.7).
    delayed: &[embalm::ability(
        ManaCost::ZERO,
        "Embalm (Exile this card from your graveyard and pay its mana cost: Create a token that's a copy of it, except it's a white Zombie in addition to its other types and has no mana cost. Embalm only as a sorcery.)",
    )
    .cost_change(its_mana_cost)
    .condition(has_a_mana_cost)],
    statics: &[static_ability(Layer::PtModify, your_of_subtype::<{ lands::sub(Subtype::Zombie) }>, plus_pt::<1, 1>, "Zombies you control get +1/+1.")],
    ..oracle::CURSECLOTH_WRAPPINGS
};
