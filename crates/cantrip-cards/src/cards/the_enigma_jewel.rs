use cantrip_api::*;
use cantrip_mechanics::triggers::YOU_ACTIVATE_A_NONMANA_ABILITY;
use cantrip_mechanics::{craft, mana, statics};
use cantrip_oracle as oracle;

/// "Nonlands with activated abilities" (CR 702.167a-b): another permanent you control or a card in your graveyard that is
/// not a land and has an activated ability (CR 602.1) - a mana ability, a loyalty ability, equip or craft among them; a
/// special action is no ability (CR 116.2).
fn nonland_with_activated_ability(q: &dyn Query, _: &Asker, id: ObjectId) -> bool {
    let c = q.chars(id);
    !c.is(CardType::Land) && c.abilities().any(|a| q.activated_ability(a).is_some_and(|d| !d.special_action))
}

static FOUR_OR_MORE: Materials = craft::materials(4, true, nonland_with_activated_ability, "nonland with activated abilities");

/// "Locus of Enlightenment has each activated ability of the exiled cards used to craft it" (CR 702.167c, 613.1f): the
/// cards still in exile, each ability Locus's own, so Locus is its source and an ability that names its card means Locus
/// (the card's ruling).
fn crafted_abilities(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let cards: Vec<_> = craft::used_to_craft(view.state(), source).into_iter().map(|c| (c, view.state().obj(c).card)).collect();
    statics::grant_activated_abilities_from(view, &cards, chars);
}

/// "You may activate each of those abilities only once each turn" (CR 602.5b): Locus has one instance of `ability` for each
/// exiled card used to craft it that prints it.
fn once_each_turn(q: &dyn Query, locus: ObjectId, ability: AbilityRef) -> usize {
    craft::used_to_craft(q.state(), locus).into_iter().filter(|&c| q.obj(c).card == ability.card).count()
}

/// "Whenever you activate an ability that isn't a mana ability, copy it. You may choose new targets for the copy." (CR
/// 707.10, 707.10c) The ability the trigger is about, copied even if it has left the stack (as it last was there).
fn copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.copy_ability(item.subject, item.controller, true)?;
    Ok(())
}

pub static BACK: CardDef = CardDef {
    abilities: &[triggered(YOU_ACTIVATE_A_NONMANA_ABILITY, copy_it, "Whenever you activate an ability that isn't a mana ability, copy it. You may choose new targets for the copy.")],
    statics: &[static_ability(Layer::Ability, statics::itself, crafted_abilities, "Locus of Enlightenment has each activated ability of the exiled cards used to craft it. You may activate each of those abilities only once each turn.")],
    granted_once_each_turn: Some(once_each_turn),
    ..oracle::LOCUS_OF_ENLIGHTENMENT
};

pub static CARD: CardDef = CardDef {
    enters: Some(mana::enters_tapped),
    abilities: &[
        mana_ability(TAP, add_n(Mana::C, 2).only_for(ManaRestriction::ABILITIES), "{T}: Add {C}{C}. Spend this mana only to activate abilities."),
        craft::ability(cantrip_api::mana("{8}{U}"), &FOUR_OR_MORE, "Craft with four or more nonlands with activated abilities {8}{U}"),
    ],
    back: Some(&BACK),
    ..oracle::THE_ENIGMA_JEWEL
};
