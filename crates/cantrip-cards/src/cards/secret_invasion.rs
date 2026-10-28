use cantrip_api::*;
use cantrip_mechanics::attach::attached_object;
use cantrip_mechanics::statics::grant_delayed_ability;
use cantrip_mechanics::triggers::{THIS_ENTERS, ward};
use cantrip_mechanics::{exile_target_until_leaves, targets};
use cantrip_oracle as oracle;

/// "Target creature other than enchanted creature" (CR 115.1): any creature on the battlefield, whoever controls it, but the
/// one this Aura is attached to.
fn creature_other_than_enchanted(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::CREATURE.filter)(q, asker, t) && matches!(t, Target::Object(id) if q.obj(asker.source).attached_to != Some(id))
}

/// "Exile up to one target creature other than enchanted creature until this Aura leaves the battlefield. Enchanted creature
/// becomes a copy of that creature until this Aura leaves the battlefield." (CR 610.3, 707.2, 611.2b): the exiled creature's
/// copiable values as it is on the battlefield as this resolves, taken before it goes; with nothing exiled, or the Aura
/// already gone, there is nothing for the enchanted creature to become a copy of for as long as the Aura stays (CR 610.3b).
fn exile_and_copy(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(target) = targets::object(g, item, 0) else { return Ok(()) };
    let Some(enchanted) = g.obj(item.object).attached_to.filter(|_| g.is_on_battlefield(item.object)) else { return Ok(()) };
    let values = g.copiable_values(target);
    exile_target_until_leaves(g, item, 0);
    g.end_sentence();
    g.become_copy(item.object, enchanted, values, Duration::WhileSourceOnBattlefield);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_YOU_CONTROL),
    abilities: &[triggered(
        THIS_ENTERS,
        exile_and_copy,
        "When this Aura enters, exile up to one target creature other than enchanted creature until this Aura leaves the battlefield. Enchanted creature becomes a copy of that creature until this Aura leaves the battlefield.",
    )
    .targets(&[up_to_one(creature_other_than_enchanted)])],
    delayed: &[ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)")],
    statics: &[static_ability(Layer::Ability, attached_object, grant_delayed_ability, "Enchanted creature has ward {2}.")],
    ..oracle::SECRET_INVASION
};
