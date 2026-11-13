use cantrip_api::*;
use cantrip_mechanics::{targets, triggers::ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST};
use cantrip_oracle as oracle;

/// The upkeep player may sacrifice the artifact that was enchanted when the ability triggered; otherwise the Aura deals
/// that player 2 damage, even if the Aura or artifact has since left (CR 118.12a, 608.2h, 701.21a).
fn sacrifice_or_damage(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    let Some(artifact) = item.subject.object() else { return Ok(()) };
    let paid = g.may_pay(player, artifact, artifact, SACRIFICE_THIS, &PAY_QUESTION)?;
    g.end_sentence();
    if !paid {
        g.damage(item.object, Target::Player(player), 2);
    }
    Ok(())
}

static PAY_QUESTION: Question = Question::new(
    "Sacrifice", "Curse Artifact: sacrifice {source}?", "If you don't, the Aura deals 2 damage to you.",
    "Sacrifice {subject}", "Don't sacrifice",
);

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        ENCHANTED_PERMANENTS_CONTROLLERS_UPKEEP_HOST,
        sacrifice_or_damage,
        "At the beginning of the upkeep of enchanted artifact's controller, this Aura deals 2 damage to that player unless they sacrifice that artifact.",
    )],
    enchant: Some(targets::ARTIFACT),
    ..oracle::CURSE_ARTIFACT
};
