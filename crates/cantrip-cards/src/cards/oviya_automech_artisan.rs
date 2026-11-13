use cantrip_api::*;
use cantrip_mechanics::{hand, statics};
use cantrip_oracle as oracle;

/// "Each creature that's attacking one of your opponents" (CR 506.4, 508.5): a creature, whoever controls it, attacking a
/// player who is an opponent of this permanent's controller. One attacking a planeswalker or a battle is attacking no player, so
/// it is not one; neither is one attacking you. Re-asked on every pass, so a creature put onto the battlefield attacking has
/// trample and one removed from combat loses it.
fn creature_attacking_an_opponent_of_yours(view: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    let state = view.state();
    so_far.is(CardType::Creature)
        && state.attackers.iter().any(|a| {
            a.id == affected && matches!(a.target, Target::Player(p) if state.opponents(state.obj(source).controller).any(|o| o == p))
        })
}

/// "You may put a creature or Vehicle card from your hand onto the battlefield. If you put an artifact onto the battlefield
/// this way, put two +1/+1 counters on it." The card is chosen as the ability resolves (CR 608.2d); "Vehicle" is the word of
/// this card's text (CR 612.1). The counters go on the object if it is still an artifact on the battlefield, which a Vehicle always is
/// and a creature card is if it is an artifact creature (CR 205.2a); a card that left the battlefield first is not put onto
/// anything.
fn put_a_creature_or_vehicle_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let vehicle = g.words(item.object).subtype(Subtype::Vehicle);
    let put = hand::may_put_onto_battlefield_judged(g, item.controller, item.object, |g, card| {
        let chars = g.chars(card);
        chars.is(CardType::Creature) || chars.has_subtype(vehicle)
    })?;
    g.end_sentence();
    if let Some(card) = put
        && g.state().obj(card).zone.is(ZoneKind::Battlefield)
        && g.chars(card).is(CardType::Artifact)
    {
        g.add_counters(card, CounterKind::PlusOne, 2);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        creature_attacking_an_opponent_of_yours,
        statics::have_trample,
        "Each creature that's attacking one of your opponents has trample.",
    )],
    abilities: &[ability(
        cost("{G}").and(TAP),
        put_a_creature_or_vehicle_card,
        "{G}, {T}: You may put a creature or Vehicle card from your hand onto the battlefield. If you put an artifact onto the battlefield this way, put two +1/+1 counters on it.",
    )],
    ..oracle::OVIYA_AUTOMECH_ARTISAN
};
