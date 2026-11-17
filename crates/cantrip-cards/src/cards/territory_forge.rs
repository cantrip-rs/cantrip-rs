use cantrip_api::*;
use cantrip_mechanics::{statics, targets, triggers::THIS_ENTERS_IF_CAST};
use cantrip_oracle as oracle;

/// "Target artifact or land" (CR 115.1): a permanent that is either.
fn artifact_or_land(q: &dyn Query, _: &Asker, t: Target) -> bool {
    matches!(t, Target::Object(id) if targets::permanent(q, id) && (q.chars(id).is(CardType::Artifact) || q.chars(id).is(CardType::Land)))
}

/// "When this artifact enters, if you cast it, exile target artifact or land." The card is exiled with Territory Forge (CR 607.2a),
/// so the static ability below finds it again; if the target is gone, nothing is exiled.
fn exile_target(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    g.exile_with(&[target], g.linked(item), false);
    Ok(())
}

/// "This artifact has all activated abilities of the exiled card" (CR 602.1, 613.1f). Each is the exiled card's own ability as
/// written, so "this artifact" in it means Territory Forge (the ruling); an exiled basic land's mana ability (CR 305.6) is one of them.
fn grant_abilities(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let exiled = view.exiled_with(view.linked_to(source));
    let cards: Vec<_> = exiled.iter().map(|&c| (c, view.state().obj(c).card)).collect();
    statics::grant_activated_abilities_from(view, &cards, chars);
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS_IF_CAST, exile_target, "When this artifact enters, if you cast it, exile target artifact or land.").targets(&[target(artifact_or_land)])],
    statics: &[static_ability(Layer::Ability, statics::itself, grant_abilities, "This artifact has all activated abilities of the exiled card.")],
    ..oracle::TERRITORY_FORGE
};
