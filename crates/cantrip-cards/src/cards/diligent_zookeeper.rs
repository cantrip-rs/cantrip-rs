use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Each non-Human creature you control" (CR 613.1f, 205.3m): every creature under the source's controller's control, itself included
/// if it is one, that the earlier layers did not make a Human (the word read through the source's text, CR 612.2).
fn non_human_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    so_far.is(CardType::Creature) && !so_far.has_subtype(q.words(source).subtype(Subtype::Human)) && q.controller(affected) == q.controller(source)
}

/// "+1/+1 for each of its creature types, to a maximum of 10" (CR 613.4c, layer 7c): the creature types it has once the type-changing
/// layers have applied, read through the view as the layer stands; a creature with eleven creature types still gets only +10/+10.
fn plus_one_per_creature_type_up_to_ten(q: &dyn LayerQuery, _: ObjectId, creature: ObjectId, chars: &mut Chars) {
    let n = q.chars(creature).subtypes.iter().filter(|&kind| SubtypeSet::CREATURE_TYPES.has(kind)).count().min(10) as i32;
    chars.power = chars.power.map(|v| v + n);
    chars.toughness = chars.toughness.map(|v| v + n);
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability_for_each(
        Layer::PtModify,
        non_human_creatures_you_control,
        plus_one_per_creature_type_up_to_ten,
        "Each non-Human creature you control gets +1/+1 for each of its creature types, to a maximum of 10.",
    )],
    ..oracle::DILIGENT_ZOOKEEPER
};
