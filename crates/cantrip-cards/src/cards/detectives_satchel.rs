use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{history, tokens};
use cantrip_oracle as oracle;

/// "Investigate twice." (CR 701.16a)
fn investigate_twice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::investigate(g, item.controller)?;
    tokens::investigate(g, item.controller)
}

/// "Activate only if you've sacrificed an artifact this turn" (CR 602.5b, 701.21a): `history::sacrificed_this_turn` of an artifact,
/// judged as it left the battlefield (CR 608.2h), a token or not. An opponent's sacrifice, an earlier turn's, and a sacrificed land
/// or creature that was no artifact do not count.
fn you_sacrificed_an_artifact_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::sacrificed_this_turn(q.state(), q.history(), asker.controller, |l| l.types.has(CardType::Artifact))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, investigate_twice, "When this artifact enters, investigate twice."),
        ability(
            cost("{T}"),
            tokens::create_a_thopter,
            "{T}: Create a 1/1 colorless Thopter artifact creature token with flying. Activate only if you've sacrificed an artifact this turn.",
        )
        .condition(you_sacrificed_an_artifact_this_turn),
    ],
    ..oracle::DETECTIVES_SATCHEL
};

pub static EXTRA: &[&CardDef] = &[&tokens::CLUE, &tokens::THOPTER];
