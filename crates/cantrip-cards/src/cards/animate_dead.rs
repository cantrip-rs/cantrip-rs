use cantrip_api::*;
use cantrip_mechanics::{attach::attached_object, targets, triggers::THIS_ENTERS};
use cantrip_oracle as oracle;

/// It loses "enchant creature card in a graveyard" and gains "enchant creature put onto the battlefield with Animate Dead"
/// (CR 613.1f, layer 6, kept as long as it is this object). Return enchanted creature card to the battlefield under your
/// control and attach this Aura to it. A card that can't enter leaves the Aura on a card it may no longer enchant, and the
/// state-based actions put it into its owner's graveyard (CR 303.4c, 704.5m).
fn return_enchanted(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::LoseEnchant(item.ability.card), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::Enchant(item.ability.card), Duration::Indefinite);
    });
    let Some(card) = g.obj(item.object).attached_to else { return Ok(()) };
    g.put_onto_battlefield_with_aura(card, item.controller, item.object, false)?;
    Ok(())
}

/// That creature's controller sacrifices it.
fn sacrifice_creature(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let entered: Vec<ObjectId> = g
        .state()
        .battlefield()
        .iter()
        .copied()
        .filter(|&id| g.obj(id).entered_with == Some(item.object))
        .collect();
    let each: Vec<(PlayerId, ObjectId)> = entered.into_iter().map(|id| (g.controller(id), id)).collect();
    g.sacrifice_each(&each);
    Ok(())
}

fn minus_one_power(_: &dyn LayerQuery, _: ObjectId, chars: &mut Chars) {
    chars.power = chars.power.map(|v| v - 1);
}

pub static CARD: CardDef = CardDef {
    enchant: Some(targets::CREATURE_CARD_IN_A_GRAVEYARD),
    becomes_aura_enchant: Some(targets::CREATURE_PUT_ONTO_THE_BATTLEFIELD_WITH_THIS),
    abilities: &[
        triggered(THIS_ENTERS, return_enchanted, "When Animate Dead enters, if it's on the battlefield, it loses \"enchant creature card in a graveyard\" and gains \"enchant creature put onto the battlefield with Animate Dead.\" Return enchanted creature card to the battlefield under your control and attach Animate Dead to it.")
            .condition(cantrip_mechanics::triggers::this_is_on_the_battlefield),
        triggered(cantrip_mechanics::triggers::THIS_LEAVES, sacrifice_creature, "When Animate Dead leaves the battlefield, that creature's controller sacrifices it."),
    ],
    statics: &[static_ability(Layer::PtModify, attached_object, minus_one_power, "Enchanted creature gets -1/-0.")],
    ..oracle::ANIMATE_DEAD
};
