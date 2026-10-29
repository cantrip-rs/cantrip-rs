use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Exile it; it is exiled with this artifact (CR 607.2a), where the second ability finds it.
fn exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(card) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_with(&[card], g.linked(item), false);
    Ok(())
}

/// "Each of them is a 1/1 Spirit in addition to its other types" (CR 614.12, 613.4b, 205.1a): each enters as that, so what
/// watches creatures enter sees a 1/1 Spirit and not the card's own power and toughness. The subtype is read through the
/// words the Vacuum last had (CR 612.1, 113.7a), as the sacrificed artifact's ability is.
static AS_A_SPIRIT: EntersAs = EntersAs { subtypes: &[Subtype::Spirit], power_toughness: Some((1, 1)), ..EntersAs::NONE };

/// Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it
/// (CR 614.1c, 122.6), all at once (CR 110.2a: so each sees the others enter, CR 603.6a), each as a 1/1 Spirit in addition to
/// its other types. The artifact was sacrificed as a cost, and the exile link outlives it (CR 113.7a).
fn release(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId)> =
        g.exiled_with(g.linked(item)).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).map(|id| (id, item.controller)).collect();
    g.put_all_onto_battlefield_becoming(&cards, item, &AS_A_SPIRIT, &[(CounterKind::Flying, 1)])?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(TAP, exile_it, "{T}: Exile target card from a graveyard.")
            .targets(&[cantrip_mechanics::targets::CARD_IN_A_GRAVEYARD]),
        ability(cost("{6}, {T}").and(SACRIFICE_THIS), release, "{6}, {T}, Sacrifice this artifact: Put each creature card exiled with this artifact onto the battlefield under your control with a flying counter on it. Each of them is a 1/1 Spirit in addition to its other types. Activate only as a sorcery.")
            .sorcery_speed(),
    ],
    ..oracle::GHOST_VACUUM
};
