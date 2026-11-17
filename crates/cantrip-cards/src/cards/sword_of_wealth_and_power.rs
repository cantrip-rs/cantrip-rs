use cantrip_api::*;
use cantrip_mechanics::attach::{attached_object, equip};
use cantrip_mechanics::{copy, delayed, protection, statics, tokens, triggers};
use cantrip_oracle as oracle;

/// "Equipped creature ... has protection from instants and from sorceries." (CR 702.16, layer 6): this static grants only the
/// protection part; the +2/+2 lives on its own PtModify static.
fn protection_from_instants_and_sorceries(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    protection::from_type_apply::<{ CardType::Instant as u8 }>(view, source, chars);
    protection::from_type_apply::<{ CardType::Sorcery as u8 }>(view, source, chars);
}

/// "Whenever equipped creature deals combat damage to a player, create a Treasure token. When you next cast an instant or sorcery
/// spell this turn, copy that spell." (CR 608.2c, 603.7b): the Treasure first, then the delayed ability, number 0 of the card's
/// `delayed` list, that triggers once on the next such spell its controller casts this turn.
fn create_a_treasure_then_watch_for_the_next_spell(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_a_treasure(g, item)?;
    delayed::watch_for_the_next(g, item)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::EQUIPPED_CREATURE_DEALS_COMBAT_DAMAGE_TO_PLAYER,
            create_a_treasure_then_watch_for_the_next_spell,
            "Whenever equipped creature deals combat damage to a player, create a Treasure token. When you next cast an instant or sorcery spell this turn, copy that spell. You may choose new targets for the copy.",
        ),
        equip(mana("{2}"), "Equip {2}"),
    ],
    delayed: &[copy::COPY_THE_NEXT_INSTANT_OR_SORCERY_YOU_CAST],
    statics: &[
        static_ability(Layer::PtModify, attached_object, statics::plus_pt::<2, 2>, "Equipped creature gets +2/+2."),
        static_ability(Layer::Ability, attached_object, protection_from_instants_and_sorceries, "has protection from instants and from sorceries."),
    ],
    ..oracle::SWORD_OF_WEALTH_AND_POWER
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
