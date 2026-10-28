use cantrip_api::*;
use cantrip_mechanics::{effects, triggers::LANDFALL};
use cantrip_oracle as oracle;

const ASK: Question = Question::new("BecomeBasePt", "have this creature's base power and toughness become 4/2 until end of turn?", "", "Become 4/2", "Don't");

/// "You may have this creature's base power and toughness become 4/2 until end of turn" (CR 613.4b, 611.2a): chosen as the
/// ability resolves; a layer 7b effect from now on, so it overwrites earlier setting effects and later ones overwrite it
/// (CR 613.7), while counters and pumps still apply to the new base (CR 613.4c).
fn become_four_two(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        g.add_effect(item.object, item.object, EffectKind::SetPt(4, 2), Duration::EndOfTurn);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        LANDFALL,
        become_four_two,
        "Landfall \u{2014} Whenever a land you control enters, you may have this creature's base power and toughness become 4/2 until end of turn.",
    )],
    ..oracle::MIRKWOOD_MEDITATOR
};
