use cantrip_api::*;
use cantrip_mechanics::{attach, targets, triggers};
use cantrip_oracle as oracle;

/// It becomes an Aura with "enchant creature put onto the battlefield with Necromancy": the Aura subtype (CR 613.1d) and the
/// enchant ability (CR 613.1f, `becomes_aura_enchant`), kept as long as it is this object. Put target creature card from a
/// graveyard onto the battlefield under your control and attach Necromancy to it, as part of the same move.
fn reanimate(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::AddSubtype(Subtype::Aura), Duration::Indefinite);
        g.add_effect(item.object, item.object, EffectKind::Enchant(item.ability.card), Duration::Indefinite);
    });
    if let Some(card) = targets::object(g, item, 0) {
        g.put_onto_battlefield_with_aura(card, item.controller, item.object, false)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    cast_as_though_flash: true,
    untimely_cleanup_rider: true,
    becomes_aura_enchant: Some(targets::CREATURE_PUT_ONTO_THE_BATTLEFIELD_WITH_THIS),
    abilities: &[
        triggered(triggers::THIS_ENTERS, reanimate, "When this enchantment enters, if it's on the battlefield, it becomes an Aura with \"enchant creature put onto the battlefield with Necromancy.\" Put target creature card from a graveyard onto the battlefield under your control and attach this enchantment to it.")
            .targets(&[targets::CREATURE_CARD_IN_A_GRAVEYARD])
            .condition(triggers::this_is_on_the_battlefield),
        triggered(triggers::THIS_LEAVES, attach::sacrifice_creature_put_onto_the_battlefield_with_this, "When this enchantment leaves the battlefield, that creature's controller sacrifices it."),
    ],
    ..oracle::NECROMANCY
};
