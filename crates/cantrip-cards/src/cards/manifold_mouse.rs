use cantrip_api::*;
use cantrip_mechanics::{effects, lands, offspring, targets, triggers};
use cantrip_oracle as oracle;

/// "Target Mouse you control gains your choice of double strike or trample until end of turn." The choice is made as the ability
/// resolves (CR 608.2d), so it is not asked of a target that is no longer legal (CR 608.2b).
fn gain_choice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(mouse) = targets::object(g, item, 0) else { return Ok(()) };
    effects::gain_your_choice_until_end_of_turn(g, item, &CARD, mouse, &[Keyword::DoubleStrike, Keyword::Trample])
}

pub static CARD: CardDef = CardDef {
    kicker: Some(offspring::kicker(mana("{2}"))),
    abilities: &[
        offspring::trigger("Offspring {2} (You may pay an additional {2} as you cast this spell. If you do, when this creature enters, create a 1/1 token copy of it.)"),
        triggered(
            triggers::BEGINNING_OF_YOUR_COMBAT,
            gain_choice,
            "At the beginning of combat on your turn, target Mouse you control gains your choice of double strike or trample until end of turn.",
        )
        .targets(&[target(targets::permanent_of_subtype_you_control::<{ lands::sub(Subtype::Mouse) }>)]),
    ],
    labels: &["Double strike", "Trample"],
    ..oracle::MANIFOLD_MOUSE
};
