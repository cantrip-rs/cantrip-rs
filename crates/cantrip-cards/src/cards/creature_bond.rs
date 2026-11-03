use cantrip_api::*;
use cantrip_mechanics::{targets, triggers};
use cantrip_oracle as oracle;

/// "When enchanted creature dies" (CR 603.6c, 603.10a): retain the old object for last-known information.
fn when_enchanted_creature_dies(
    q: &dyn Query,
    asker: &Asker,
    event: &Event,
) -> Option<Fired> {
    let (old, _) = triggers::enchanted_creature_died(q, asker.source, event)?;
    Fired::about(Target::Object(old))
}

/// A missing toughness is zero (CR 208.5); negative toughness cannot cause negative damage (CR 107.1b).
fn damage_for_toughness(toughness: Option<i16>) -> u16 {
    toughness.unwrap_or_default().max(0) as u16
}

/// "When enchanted creature dies, this Aura deals damage equal to that creature's toughness to the creature's controller." (CR 608.2, 120.3).
fn aura_deals_damage_equal_creatures(
    g: &mut dyn Game,
    item: &StackItem,
) -> Result<(), Suspend> {
    if let Some(creature) = item.subject.object() {
        let amount = damage_for_toughness(g.chars(creature).toughness);
        let controller = g.controller(creature);
        g.damage(item.object, Target::Player(controller), amount);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            when_asked(EventMask::ZONE_CHANGE, when_enchanted_creature_dies),
            aura_deals_damage_equal_creatures,
            "When enchanted creature dies, this Aura deals damage equal to that creature's toughness to the creature's controller.",
        ),
    ],
    enchant: Some(targets::CREATURE),
    ..oracle::CREATURE_BOND
};

#[cfg(test)]
#[path = "tests/creature_bond.rs"]
mod tests;
