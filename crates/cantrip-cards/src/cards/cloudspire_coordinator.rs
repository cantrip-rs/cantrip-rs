use cantrip_api::*;
use cantrip_mechanics::tokens::{self, PILOT_SADDLES_AND_CREWS};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// "Create X 1/1 colorless Pilot creature tokens, where X is the number of Mounts and/or Vehicles that entered the battlefield under your
/// control this turn" (CR 111.1, 603.6a): each permanent that entered under the ability's controller this turn as a Mount or a Vehicle
/// (or both, once), as it entered, whatever it has become or wherever it has gone since, tokens included. The types are words of this
/// card's text (CR 612).
fn pilots_for_each_mount_or_vehicle_entered(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let q: &dyn Query = g;
    let words = q.words(item.object);
    let (mount, vehicle) = (words.subtype(Subtype::Mount), words.subtype(Subtype::Vehicle));
    let x = q
        .history()
        .entered()
        .filter(|e| e.controller == item.controller && (e.chars(q).has_subtype(mount) || e.chars(q).has_subtype(vehicle)))
        .count();
    tokens::create(g, item.controller, &PILOT_SADDLES_AND_CREWS, x as u32)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, effects::scry_two, "When this creature enters, scry 2."),
        ability(
            TAP,
            pilots_for_each_mount_or_vehicle_entered,
            "{T}: Create X 1/1 colorless Pilot creature tokens, where X is the number of Mounts and/or Vehicles that entered the battlefield under your control this turn. The tokens have \"This token saddles Mounts and crews Vehicles as though its power were 2 greater.\"",
        ),
    ],
    ..oracle::CLOUDSPIRE_COORDINATOR
};

pub static EXTRA: &[&CardDef] = &[&PILOT_SADDLES_AND_CREWS];
