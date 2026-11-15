use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}." (CR 608.2, 701.21a, 118.1).
fn sacrifice_enchantment_unless_pay_ww(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::sacrifice_this_unless_pay(g, item, cost("{W}{W}"))
}

/// "Sacrifice a Swamp": a permanent with the land type its text names Swamp (CR 205.3i), read through the words of this
/// enchantment's text (CR 612.2), which the additional cost is asked as (`CostAddition`).
fn swamp(q: &dyn Query, asker: &Asker, id: ObjectId) -> bool {
    q.chars(id).has_subtype(q.words(asker.source).subtype(Subtype::Swamp))
}

/// An additional "Sacrifice a Swamp" for each of `black` black mana symbols (CR 118.8, 701.21a): as many Swamps, sacrificed
/// as the total cost is paid (CR 601.2h). None where there is no such symbol.
fn swamp_for_each(black: u32) -> Option<PermanentCost> {
    if black == 0 {
        return None;
    }
    Cost::sacrifice(black, swamp).permanents
}

/// "Spells cost an additional "Sacrifice a Swamp" to cast for each black mana symbol in their mana costs." (CR 118.8,
/// 601.2f): every spell, whoever casts it, by the symbols of its mana cost as a spell (CR 202.1), not what is paid for it - a
/// hybrid symbol with a black half, whichever half is paid (the ruling), and a black Phyrexian symbol among them (CR 107.4e-f).
fn spells_cost_swamps(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed) -> Option<PermanentCost> {
    let Costed::Spell(spell) = costed else { return None };
    swamp_for_each(q.mana_cost(spell).symbols_of(q.words(source).color(Color::Black)))
}

/// "Activated abilities cost an additional "Sacrifice a Swamp" to activate for each black mana symbol in their activation
/// costs." (CR 118.8, 602.2b): every activated ability, mana abilities included (CR 605.1a), whoever activates it, by the
/// symbols of the activation cost it prints (CR 602.1a). A special action is no activated ability (CR 116.1).
fn abilities_cost_swamps(q: &dyn Query, source: ObjectId, _: PlayerId, costed: Costed) -> Option<PermanentCost> {
    let (Costed::Ability(_, ability, _) | Costed::PlayerAbility(_, ability, _)) = costed else { return None };
    let def = q.activated_ability(ability)?;
    swamp_for_each(def.cost.mana.symbols_of(q.words(source).color(Color::Black)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_UPKEEP,
        sacrifice_enchantment_unless_pay_ww,
        "At the beginning of your upkeep, sacrifice this enchantment unless you pay {W}{W}.",
    )],
    cost_additions: &[spells_cost_swamps, abilities_cost_swamps],
    ..oracle::DROUGHT
};
