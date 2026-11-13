use cantrip_api::*;
use cantrip_mechanics::tokens::{self, PILOT_SADDLES_AND_CREWS};
use cantrip_mechanics::{effects, targets};
use cantrip_oracle as oracle;

/// "Target creature or Vehicle card from your graveyard" (CR 115.1, 205.3g): a card in the asker's own graveyard with the
/// creature type or the Vehicle subtype.
fn creature_or_vehicle_card_in_your_graveyard(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if targets::is_creature_or_vehicle(&q.chars(id)))
}

/// "Return target creature or Vehicle card from your graveyard to the battlefield. Create a 1/1 colorless Pilot creature token
/// with \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\"" (CR 111.1, 608.2c).
fn return_then_create_a_pilot(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::return_first_target_to_battlefield(g, item)?;
    g.end_sentence();
    tokens::create_a_pilot(g, item)
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[target(creature_or_vehicle_card_in_your_graveyard)], return_then_create_a_pilot),
    ..oracle::BACK_ON_TRACK
};

pub static EXTRA: &[&CardDef] = &[&PILOT_SADDLES_AND_CREWS];
