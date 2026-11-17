use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_DRAW_YOUR_SECOND_CARD;
use cantrip_oracle as oracle;

/// "Whenever you draw your second card each turn, this creature loses defender and gains vigilance until end of turn." (CR 121.1, 603.2,
/// 611.2a, 702.3, 702.20): both effects last the rest of the turn from this creature as the ability resolves; a creature that has
/// left the battlefield since is a new object and gets neither (CR 400.7).
fn loses_defender_and_gains_vigilance(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::LoseKeywords(Keywords::of(&[Keyword::Defender])), Duration::EndOfTurn);
        g.add_effect(item.object, item.object, EffectKind::AddKeywords(Keywords::of(&[Keyword::Vigilance])), Duration::EndOfTurn);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        YOU_DRAW_YOUR_SECOND_CARD,
        loses_defender_and_gains_vigilance,
        "Whenever you draw your second card each turn, this creature loses defender and gains vigilance until end of turn.",
    )],
    ..oracle::JADED_ANALYST
};
