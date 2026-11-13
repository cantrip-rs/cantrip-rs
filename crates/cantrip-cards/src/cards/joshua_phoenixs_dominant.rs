use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, exile_and_return, exile_and_return_transformed, hand, select, targets};
use cantrip_oracle as oracle;

/// "Discard up to two cards, then draw that many cards."
fn loot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    hand::discard_up_to_then_draw(g, item.controller, item.object, 2)
}

/// "Exile Joshua, then return it to the battlefield transformed under its owner's control" (CR 712.14a): the new object is
/// Phoenix, Warden of Fire, a Saga that enters with a lore counter (CR 714.3a). Nothing returns if Joshua has left the
/// battlefield already (CR 400.7).
fn transform(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let owner = g.obj(item.object).owner;
    exile_and_return_transformed(g, item.object, owner)
}

/// III: "Return any number of target creature cards with total mana value 6 or less from your graveyard to the battlefield.
/// Exile Phoenix, then return it to the battlefield (front face up)." The total was judged as the targets were chosen and is
/// judged again on resolution: if it has grown past 6 every target is illegal (CR 608.2b); otherwise the targets still legal
/// enter under their owner's control (CR 110.2a). Then the second sentence (CR 608.2c): Phoenix, if it is still on the
/// battlefield, returns as a new object with its front face up (CR 712.14a).
fn flames_of_rebirth(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let cards: Vec<(ObjectId, PlayerId)> = targets::legal_objects(g, item).into_iter().map(|c| (c, item.controller)).collect();
    g.put_all_onto_battlefield(&cards)?;
    exile_and_return(g, &[item.object])
}

/// Flying and lifelink are core keywords read from its characteristics.
pub static BACK: CardDef = CardDef {
    abilities: &[
        chapter(1, effects::damage_each_opponent_n::<2>, "I \u{2014} Rising Flames \u{2014} Phoenix deals 2 damage to each opponent."),
        chapter(2, effects::damage_each_opponent_n::<2>, "II \u{2014} Rising Flames \u{2014} Phoenix deals 2 damage to each opponent."),
        chapter(3, flames_of_rebirth, "III \u{2014} Flames of Rebirth \u{2014} Return any number of target creature cards with total mana value 6 or less from your graveyard to the battlefield. Exile Phoenix, then return it to the battlefield (front face up).")
            .targets(&[TargetSpec { min: 0, max: TargetSpec::ANY_NUMBER, ..targets::CREATURE_CARD_IN_YOUR_GRAVEYARD }
                .together(select::total_mana_value_at_most(6))
                .labelled("Creature cards from your graveyard to return to the battlefield")]),
    ],
    ..oracle::PHOENIX_WARDEN_OF_FIRE
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, loot, "When Joshua enters, discard up to two cards, then draw that many cards."),
        ability(
            cost("{3}{R}{W}").and(TAP),
            transform,
            "{3}{R}{W}, {T}: Exile Joshua, then return it to the battlefield transformed under its owner's control. Activate only as a sorcery.",
        )
        .sorcery_speed(),
    ],
    back: Some(&BACK),
    ..oracle::JOSHUA_PHOENIXS_DOMINANT
};
