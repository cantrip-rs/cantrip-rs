use cantrip_api::*;
use cantrip_mechanics::ferocious;
use cantrip_oracle as oracle;

/// "This creature can attack as though it didn't have defender as long as you control a creature with power 4 or greater"
/// (CR 702.3b, 609.4): only this creature, and only while its controller controls a creature with power 4 or greater (itself
/// included - it has power 3 and gets no bigger on its own). It keeps defender for everything else. Defender is a core keyword
/// read from the card's characteristics.
fn may_attack_with_a_big_creature(q: &dyn Query, asker: &Asker, creature: ObjectId) -> bool {
    creature == asker.source && ferocious::you_control_asked(q, asker)
}

pub static CARD: CardDef = CardDef { attacks_despite_defender: &[may_attack_with_a_big_creature], ..oracle::BRISTLEPACK_SENTRY };
