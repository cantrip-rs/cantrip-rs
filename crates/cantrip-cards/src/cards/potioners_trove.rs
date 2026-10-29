use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Activate only if you've cast an instant or sorcery spell this turn" (CR 602.5b): asked of the history as the ability is
/// activated: a spell cast earlier this turn by its controller that was an instant or a sorcery as it was cast, whatever has
/// become of it since (resolved, countered or still on the stack). A copy is not cast (CR 707.10) and is not in the history.
fn cast_an_instant_or_sorcery_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    q.history().casts_by(asker.controller).any(|c| c.is(CardType::Instant) || c.is(CardType::Sorcery))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        ability(TAP, |g, item| effects::gain_life(g, item, 2), "{T}: You gain 2 life. Activate only if you've cast an instant or sorcery spell this turn.")
            .condition(cast_an_instant_or_sorcery_this_turn),
    ],
    ..oracle::POTIONERS_TROVE
};
