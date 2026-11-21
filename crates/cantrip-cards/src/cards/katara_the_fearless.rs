use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If a triggered ability of an Ally you control triggers, that ability triggers an additional time." (CR 603.2d): the
/// ability's source a permanent under Katara's controller with the Ally subtype as it is when the ability triggers (as it last
/// existed for one that looks back), Katara herself among them; Ally is a word of her text (CR 612). A replacement effect is no
/// triggered ability and is not repeated (the rulings).
fn ally_you_control(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.of_a_permanent_controlled_by(asker.controller) && q.chars(triggering.source).has_subtype(q.words(asker.source).subtype(Subtype::Ally))
}

pub static CARD: CardDef = CardDef { additional_triggers: &[ally_you_control], ..oracle::KATARA_THE_FEARLESS };
