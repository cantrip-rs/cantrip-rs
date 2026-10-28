use cantrip_api::*;
use cantrip_mechanics::triggers::{ward, REPARTEE};
use cantrip_mechanics::{effects, pump};
use cantrip_oracle as oracle;

/// "This creature gets +1/+0 and gains lifelink until end of turn" (CR 611.2a): one instruction, on the creature the
/// ability came from, nothing once it has left the battlefield (CR 400.7).
fn pump_and_lifelink(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    instruction(g, |g| {
        pump(g, item.object, item.object, 1, 0);
        effects::gain_lifelink_until_end_of_turn(g, item.object, item.object);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ward::<2>("Ward {2} (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays {2}.)"),
        triggered(
            REPARTEE,
            pump_and_lifelink,
            "Repartee — Whenever you cast an instant or sorcery spell that targets a creature, this creature gets +1/+0 and gains lifelink until end of turn.",
        ),
    ],
    ..oracle::INKSHAPE_DEMONSTRATOR
};
