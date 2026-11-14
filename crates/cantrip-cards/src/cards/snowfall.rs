use cantrip_api::*;
use cantrip_mechanics::{choose, cumulative_upkeep, lands, mana};
use cantrip_oracle as oracle;

/// "Whenever an Island is tapped for mana" (CR 106.12a, 612.2).
fn island_tapped_for_mana(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    let mut fired = mana::a_player_taps_a_land_type_for_mana::<{ lands::sub(Subtype::Island) }>(q, asker, event)?;
    let Target::Object(island) = fired.subject else { return None };
    fired.data = [u32::from(q.controller(island).0), u32::from(q.chars(island).supertypes.snow)];
    Some(fired)
}

/// "Its controller may add an additional {U}. If that Island is snow, its controller may add an additional {U}{U} instead.
/// Spend this mana only to pay cumulative upkeep costs." (CR 605.1b, 106.6).
fn additional_upkeep_mana(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let player = PlayerId(item.data[0] as u8);
    // The snow Island's {U}{U} is added instead of the {U} (CR 614.1a): its controller adds none or {U}{U}, never {U}.
    let (label, units) = if item.data[1] != 0 { (2, 2) } else { (1, 1) };
    let choices = vec![Label::Card(item.ability.card, 0), Label::Card(item.ability.card, label)];
    if choose::one_of(g, player, item.object, choices)? > 0 {
        g.add_restricted_mana(player, Mana::U, units, ManaRestriction::CumulativeUpkeep);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        cumulative_upkeep!(cumulative_upkeep::Payment::Mana(mana("{U}")), "Cumulative upkeep {U}"),
        triggered(
            when_asked(EventMask::TAPPED, island_tapped_for_mana),
            additional_upkeep_mana,
            "Whenever an Island is tapped for mana, its controller may add an additional {U}. If that Island is snow, its controller may add an additional {U}{U} instead. Spend this mana only to pay cumulative upkeep costs.",
        ).mana_ability(TriggeredMana::Unstated),
    ],
    labels: &["Add no mana", "Add one {U}", "Add {U}{U}"],
    ..oracle::SNOWFALL
};
