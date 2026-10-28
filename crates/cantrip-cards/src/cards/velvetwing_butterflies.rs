use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Tap one or two target creatures" (CR 701.26a): those still legal as it resolves (CR 608.2b) are tapped.
fn tap_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = targets::legal_objects(g, item);
    instruction(g, |g| {
        for creature in creatures {
            g.tap(creature);
        }
    });
    Ok(())
}

/// The Adventure (CR 715): Gaze in Wonder.
pub static GAZE_IN_WONDER: CardDef = CardDef { spell: spell(&[targets::CREATURE.between(1, 2)], tap_them), ..oracle::GAZE_IN_WONDER };

pub static CARD: CardDef = CardDef { adventure: Some(&GAZE_IN_WONDER), ..oracle::VELVETWING_BUTTERFLIES };

pub static EXTRA: &[&CardDef] = &[&GAZE_IN_WONDER];
