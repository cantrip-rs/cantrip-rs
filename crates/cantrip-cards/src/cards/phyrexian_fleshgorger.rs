use cantrip_api::*;
use cantrip_mechanics::triggers::ward_life_equal_to_power;
use cantrip_oracle as oracle;

/// "Ward - Pay life equal to this creature's power" (CR 702.21a): the power read is the creature's as it is when the ward
/// ability resolves - the prototyped 3 when it was cast prototyped (CR 718.3b).
const WARD: AbilityDef = ward_life_equal_to_power("Ward\u{2014}Pay life equal to this creature's power. (Whenever this creature becomes the target of a spell or ability an opponent controls, counter it unless that player pays life equal to its power.)");

pub static CARD: CardDef = CardDef { abilities: &[WARD], ..oracle::PHYREXIAN_FLESHGORGER };
