use cantrip_api::*;
use cantrip_mechanics::triggers::{of_subtype_you_control_enters, THIS_ENTERS};
use cantrip_mechanics::{effects, history, lands, targets, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 black Bat creature token with flying for each mana from a Cave spent to cast it" (CR 601.2h, 106.3, 111.1): all
/// of the payment, additional costs and generic mana too, that a Cave produced, read from the record of the spell this permanent
/// became. A Cave is judged as it last was on the battlefield. A permanent that was not cast (put onto the battlefield by an
/// effect) had no mana spent, so none are made.
fn bats_for_mana_from_caves(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let spell = history::cast_of(g, item.object).map_or(ObjectId::NONE, |cast| cast.spell);
    let from_caves: u32 = g
        .history()
        .mana_spent_to_cast(spell)
        .filter(|&(source, _)| source != ObjectId::NONE && g.chars(source).has_subtype(Subtype::Cave))
        .map(|(_, mana)| mana.total())
        .sum();
    tokens::create(g, item.controller, &tokens::BAT, from_caves)?;
    Ok(())
}

/// Mana a Cave produced is told apart from other mana while this spell is paid for (CR 106.3, 601.2h): which mana of a kind is
/// spent is its payer's choice, and the Bats count the Cave mana among it. Asked as the spell only; on the battlefield this
/// enchantment asks nothing of later payments.
fn cave_mana_is_told_apart(q: &dyn Query, me: &Asker, source: ObjectId) -> bool {
    me.spell && q.chars(source).has_subtype(Subtype::Cave)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, bats_for_mana_from_caves, "When this enchantment enters, create a 1/1 black Bat creature token with flying for each mana from a Cave spent to cast it."),
        triggered(
            when_asked(EventMask::ZONE_CHANGE, of_subtype_you_control_enters::<{ lands::sub(Subtype::Cave) }>),
            effects::plus_one_counter_on_target,
            "Whenever a Cave you control enters, put a +1/+1 counter on target creature you control.",
        )
        .targets(&[targets::CREATURE_YOU_CONTROL]),
    ],
    mana_told_apart: &[cave_mana_is_told_apart],
    ..oracle::BAT_COLONY
};

pub static EXTRA: &[&CardDef] = &[&tokens::BAT];
