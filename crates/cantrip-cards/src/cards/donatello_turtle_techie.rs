use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

/// "If you control an artifact" (CR 603.4): asked as the ability triggers and again as it resolves, of the battlefield as it is
/// then, through the layers.
fn you_control_an_artifact(q: &dyn Query, asker: &Asker) -> bool {
    q.state().battlefield().iter().any(|&id| q.controller(id) == asker.controller && q.chars(id).is(CardType::Artifact))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, |g, item| effects::draw(g, item, 1), "When Donatello enters, if you control an artifact, draw a card.")
        .condition(you_control_an_artifact)],
    ..oracle::DONATELLO_TURTLE_TECHIE
};
