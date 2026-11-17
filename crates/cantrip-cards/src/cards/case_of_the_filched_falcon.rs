use cantrip_api::*;
use cantrip_mechanics::{artifacts_you_control, solved, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "To solve — You control three or more artifacts" (CR 719.3a): an intervening "if" (CR 603.4), asked as the end step begins and as
/// the ability resolves; artifacts the Case's controller controls, tokens and the Case's own Clue included.
fn three_or_more_artifacts(q: &dyn Query, asker: &Asker) -> bool {
    artifacts_you_control(q, asker.controller) >= 3
}

/// "Put four +1/+1 counters on target noncreature artifact. It becomes a 0/0 Bird creature with flying in addition to its other types"
/// (CR 122.1, 611.2a, 613.1d, 613.4b): the counters first, then the artifact keeps what it is, gains the creature type, the Bird
/// subtype and flying, and has base power and toughness 0/0 - with no end (the effect is not "until end of turn"), laid over its
/// printed P/T (a Vehicle's too) so the counters make it 4/4. "Bird" is a word of the Case's text (CR 612.1). Nothing happens if the
/// artifact is no longer a legal target (CR 608.2b).
fn four_counters_and_becomes_a_bird(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(artifact) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_counters(artifact, CounterKind::PlusOne, 4);
    let bird = g.words(item.object).subtype(Subtype::Bird);
    instruction(g, |g| {
        for kind in [
            EffectKind::AddTypes(TypeSet::of(&[CardType::Creature])),
            EffectKind::AddSubtype(bird),
            EffectKind::AddKeywords(Keywords::of(&[Keyword::Flying])),
            EffectKind::SetPt(0, 0),
        ] {
            g.add_effect(item.object, artifact, kind, Duration::Indefinite);
        }
    });
    Ok(())
}

/// "Investigate." (CR 701.16a): its controller creates a Clue token.
fn investigate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, investigate, "When this Case enters, investigate. (Create a Clue token. It's an artifact with \"{2}, Sacrifice this token: Draw a card.\")"),
        solved::to_solve(three_or_more_artifacts, "To solve \u{2014} You control three or more artifacts. (If unsolved, solve at the beginning of your end step.)"),
        solved::ability(
            cost("{2}{U}").and(SACRIFICE_THIS),
            four_counters_and_becomes_a_bird,
            "Solved \u{2014} {2}{U}, Sacrifice this Case: Put four +1/+1 counters on target noncreature artifact. It becomes a 0/0 Bird creature with flying in addition to its other types.",
        )
        .targets(&[targets::NONCREATURE_ARTIFACT]),
    ],
    ..oracle::CASE_OF_THE_FILCHED_FALCON
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE];
