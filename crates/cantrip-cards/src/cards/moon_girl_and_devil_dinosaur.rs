use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Until end of turn, Moon Girl and Devil Dinosaur's base power and toughness become 6/6 and they gain trample" (CR 611.2a,
/// 613.4b): counters and pumps still count on top of the 6/6.
fn become_6_6_with_trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // One sentence, one instruction (CR 608.2c).
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::SetPt(6, 6), Duration::EndOfTurn);
        effects::gain_until_end_of_turn(g, item.object, item.object, Keyword::Trample);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::YOU_DRAW_YOUR_SECOND_CARD,
            become_6_6_with_trample,
            "Whenever you draw your second card each turn, until end of turn, Moon Girl and Devil Dinosaur's base power and toughness become 6/6 and they gain trample.",
        ),
        triggered(
            triggers::ARTIFACT_YOU_CONTROL_ENTERS,
            |g, item| effects::draw(g, item, 1),
            "Whenever an artifact you control enters, draw a card. This ability triggers only once each turn.",
        )
        .triggers_each_turn(1),
    ],
    ..oracle::MOON_GIRL_AND_DEVIL_DINOSAUR
};
