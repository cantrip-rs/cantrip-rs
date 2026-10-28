use cantrip_api::*;
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// Chapter I: "Target creature you control gains hexproof for as long as this Saga remains on the battlefield." (CR 702.11,
/// 611.2b): a layer 6 grant that ends the moment the Saga leaves the battlefield, nothing if the target is illegal (CR 608.2b).
fn hexproof(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Hexproof])), Duration::WhileSourceOnBattlefield);
    Ok(())
}

/// "All damage that would be dealt by the creature" (CR 615.1, 120.1): damage of any kind whose source is the creature the
/// effect is about. Once that creature has left the battlefield it is a new object (CR 400.7), which the effect is not about.
fn damage_dealt_by(_: &dyn Query, _: &Asker, creature: ObjectId, d: &DamageEvent) -> bool {
    d.source == creature
}

const DEALT_BY: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents all damage dealt by that creature",
    damage: "damage",
    of_it: Some("dealt by it"),
    applies: damage_dealt_by,
};

/// Chapter II: "Prevent all damage that would be dealt by up to one target creature for as long as this Saga remains on the
/// battlefield." (CR 615.1, 611.2b): with no target chosen, or the target gone, nothing is prevented.
fn prevent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(
        item.object,
        creature,
        EffectKind::Prevention { card: item.ability.card, index: 0, to: Target::None, shield: Shield::All },
        Duration::WhileSourceOnBattlefield,
    );
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        chapter(1, hexproof, "I \u{2014} Target creature you control gains hexproof for as long as this Saga remains on the battlefield.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        chapter(2, prevent, "II \u{2014} Prevent all damage that would be dealt by up to one target creature for as long as this Saga remains on the battlefield.")
            .targets(&[targets::CREATURE.optional()]),
        chapter(3, |g, item| effects::draw(g, item, 1), "III, IV \u{2014} Draw a card."),
        chapter(4, |g, item| effects::draw(g, item, 1), "III, IV \u{2014} Draw a card."),
    ],
    granted_preventions: &[DEALT_BY],
    ..oracle::OLD_FAT_SPIDER_CANT_SEE_ME
};
