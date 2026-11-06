use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, targets};
use cantrip_oracle as oracle;

/// The count follows the defending player while attacking, otherwise the creature's controller.
/// Apply in layer 7c using the toughness at this point in the layer (CR 613.4c).
fn apply(q: &dyn LayerQuery, _: ObjectId, creature: ObjectId, chars: &mut Chars) {
    let state = q.state();
    let player = state
        .attackers
        .iter()
        .find(|a| a.id == creature)
        .map_or_else(|| q.controller(creature), |a| a.defending);
    let count = state
        .battlefield()
        .iter()
        .filter(|&&id| {
            let land = q.chars(id);
            q.controller(id) == player && land.supertypes.snow && land.is(CardType::Land)
        })
        .count()
        .min(i32::MAX as usize) as i32;
    let y = count.min((i32::from(q.chars(creature).toughness.unwrap_or(0)) - 1).max(0));
    chars.power = chars
        .power
        .map(|v| v.saturating_sub(count));
    chars.toughness = chars
        .toughness
        .map(|v| v.saturating_sub(y));
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability_for_each(
        Layer::PtModify,
        attached_object,
        apply,
        "Enchanted creature gets -X/-Y. If that creature is attacking, X is the number of snow lands defending player controls. Otherwise, X is the number of snow lands its controller controls. Y is equal to X or to enchanted creature's toughness minus 1, whichever is smaller.",
    )],
    enchant: Some(targets::CREATURE),
    ..oracle::SNOWBLIND
};
