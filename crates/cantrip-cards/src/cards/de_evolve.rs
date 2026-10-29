use cantrip_api::*;
use cantrip_mechanics::{attach::{attached_object, enchanted_creature_cant_block}, targets, triggers};
use cantrip_oracle as oracle;

/// "Enchanted creature gets -4/-0" (CR 613.4c, layer 7c).
fn minus_four_power(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|power| power - 4);
}

/// "... is a Salamander. (It loses all other creature types.)" Salamander is a word of this Aura's text (CR 205.3, 612; layer 4, 613.1d).
fn salamander(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let kept = chars.subtypes.iter().filter(|subtype| !subtype.is_creature_type()).fold(SubtypeSet::NONE, SubtypeSet::with);
    chars.subtypes = kept.with(view.words(source).subtype(Subtype::Salamander));
}

/// The threshold condition is an intervening "if" (CR 603.4): the core checks it as each upkeep begins and again as the
/// ability resolves. The graveyard belongs to the Aura's controller.
fn threshold(q: &dyn Query, asker: &Asker) -> bool {
    cantrip_mechanics::threshold(q, asker.controller)
}

fn exile_enchanted_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = g.obj(item.object).attached_to {
        g.exile(creature);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE),
    abilities: &[triggered(
        triggers::EACH_UPKEEP,
        exile_enchanted_creature,
        "At the beginning of each upkeep, if there are seven or more cards in your graveyard, exile enchanted creature.",
    )
    .condition(threshold)],
    block_rules: &[enchanted_creature_cant_block],
    statics: &[
        static_ability(Layer::Type, attached_object, salamander, "Enchanted creature is a Salamander."),
        static_ability(Layer::PtModify, attached_object, minus_four_power, "Enchanted creature gets -4/-0.").continues(),
    ],
    ..oracle::DE_EVOLVE
};
