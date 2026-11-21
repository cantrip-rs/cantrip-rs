use cantrip_api::*;
use cantrip_mechanics::{choose, mana, targets};
use cantrip_oracle as oracle;

/// "Put any number of permanent cards from your hand onto the battlefield." Chosen as it resolves, and put there together
/// (CR 101.4); an Aura chooses what it enchants as it enters (CR 303.4f).
fn put_permanents(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let eligible: Vec<ObjectId> =
        g.state().zone(Zone::hand(item.controller)).iter().copied().filter(|&c| targets::is_permanent_card(&g.chars(c))).collect();
    // "Any number of"; nothing is asked with no permanent card in the hand.
    let chosen = choose::any_number_of(g, item.controller, item.object, Purpose::PutOntoBattlefield, "", eligible.clone(), eligible)?;
    let moves: Vec<(ObjectId, PlayerId)> = chosen.into_iter().map(|c| (c, item.controller)).collect();
    g.put_all_onto_battlefield(&moves)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, mana::current_power_of_one_color().only_for(ManaRestriction::ABILITIES), "{T}: Add X mana of any one color, where X is Redshift's power. Spend this mana only to activate abilities."),
        ability(cost("{10}{R}{G}"), put_permanents, "Exhaust \u{2014} {10}{R}{G}: Put any number of permanent cards from your hand onto the battlefield. (Activate each exhaust ability only once.)")
            .exhaust(),
    ],
    ..oracle::REDSHIFT_ROCKETEER_CHIEF
};
