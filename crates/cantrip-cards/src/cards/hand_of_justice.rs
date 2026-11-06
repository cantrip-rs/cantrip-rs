use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Untapped white creatures controlled by the activator can be tapped for the cost (CR 118.3).
fn destroy_target_creature(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    !(q.state().obj(id).tapped)
        && q.chars(id).colors.has(q.words(asker.source).color(Color::White))
        && q.chars(id).is(CardType::Creature) && asker.controller == q.controller(id)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(
                TAP.and(tap_untapped(3, destroy_target_creature)),
                effects::destroy_first_target,
                "{T}, Tap three untapped white creatures you control: Destroy target creature.",
            )
            .targets(&[targets::CREATURE]),
    ],
    ..oracle::HAND_OF_JUSTICE
};
