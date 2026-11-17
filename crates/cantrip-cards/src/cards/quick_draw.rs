use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature you control gets +1/+1 and gains first strike until end of turn. Creatures target opponent controls lose first strike
/// and double strike until end of turn." (CR 611.2a, 702.7, 702.4) Two instructions, each skipped if its own target is illegal as this
/// resolves (CR 608.2b); the second takes the creatures the opponent controls then, and none that enter later (CR 611.2c).
fn quick_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::target_creature_gets_and_gains_first_strike::<1, 1>(g, item)?;
    g.end_sentence();
    let Some(opponent) = targets::player(g, item, 1) else { return Ok(()) };
    let creatures: Vec<ObjectId> = g.permanents_of(opponent).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let keywords = Keywords::of(&[Keyword::FirstStrike, Keyword::DoubleStrike]);
    instruction(g, |g| {
        for id in creatures {
            g.add_effect(item.object, id, EffectKind::LoseKeywords(keywords), Duration::EndOfTurn);
        }
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::OPPONENT], quick_draw),
    ..oracle::QUICK_DRAW
};
