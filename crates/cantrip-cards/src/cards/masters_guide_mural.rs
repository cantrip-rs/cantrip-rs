use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{craft, history, tokens};
use cantrip_oracle as oracle;

/// The 4/4 white and blue Golem artifact creature token both faces make.
pub static GOLEM: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::White).0 | ColorSet::of(Color::Blue).0)),
    types: TypeSet::of(&[CardType::Artifact, CardType::Creature]),
    power: Some(4),
    toughness: Some(4),
    ..token!(Golem)
};

pub static EXTRA: &[&CardDef] = &[&GOLEM];

/// "Create a 4/4 white and blue Golem artifact creature token." (CR 111.1): one, under the ability's controller.
fn create_a_golem(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &GOLEM, 1)?;
    Ok(())
}

/// "Activate only if this artifact or another artifact entered the battlefield under your control this turn" (CR 602.5b, 603.6a):
/// asked of the history, which keeps each permanent as it entered - an artifact then, under its controller then. The crafted
/// Manufactory is a new object that entered as this artifact (CR 702.167a), so it counts itself.
fn an_artifact_entered_under_your_control_this_turn(q: &dyn Query, asker: &Asker) -> bool {
    history::artifact_entered_under_control_this_turn(q.history(), asker.controller)
}

pub static BACK: CardDef = CardDef {
    abilities: &[ability(
        TAP,
        create_a_golem,
        "{T}: Create a 4/4 white and blue Golem artifact creature token. Activate only if this artifact or another artifact entered the battlefield under your control this turn.",
    )
    .condition(an_artifact_entered_under_your_control_this_turn)],
    ..oracle::MASTERS_MANUFACTORY
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, create_a_golem, "When this artifact enters, create a 4/4 white and blue Golem artifact creature token."),
        craft::ability(mana("{4}{W}{W}{U}"), &craft::ARTIFACT, "Craft with artifact {4}{W}{W}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::MASTERS_GUIDE_MURAL
};
