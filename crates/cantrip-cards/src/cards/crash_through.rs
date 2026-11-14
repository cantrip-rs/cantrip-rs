use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Creatures you control gain trample until end of turn. Draw a card." (CR 611.2c, 702.19): the creatures its controller controls as it
/// resolves, and none that enters later; a controller with no creatures just draws.
fn effect(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain(g, item, &[Keyword::Trample]);
    effects::draw(g, item, 1)
}

pub static CARD: CardDef = CardDef { spell: spell(&[], effect), ..oracle::CRASH_THROUGH };
