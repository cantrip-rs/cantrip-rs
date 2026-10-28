use cantrip_api::*;
use cantrip_mechanics::attach;
use cantrip_mechanics::library::{Find, search_to_hand};
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_oracle as oracle;

fn is_equipment(q: &dyn Query, id: ObjectId) -> bool {
    q.chars(id).has_subtype(Subtype::Equipment)
}

/// "Search your library for an Equipment card, reveal it, put it into your hand, then shuffle." (CR 701.23): the search may
/// find nothing, as any search for a quality may.
fn search(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    search_to_hand(g, item.controller, item.object, Find::ONE.revealed(), is_equipment)?;
    Ok(())
}

/// "As long as Cloud is equipped, if a triggered ability of Cloud or an Equipment attached to it triggers, that ability
/// triggers an additional time." (CR 603.2d): a triggered ability of a permanent as it is when the ability triggers, so
/// no matter who controls the Equipment (the text says no "you control"), the Equipment must be attached to Cloud now, and Cloud equipped now. An ability Cloud is granted (an Equipment's
/// "has ..." text) is Cloud's own; an Equipment attached to another creature is not covered.
fn triggers_again(q: &dyn Query, asker: &Asker, _: &Event, triggering: &Triggering) -> bool {
    triggering.permanent
        && attach::is_equipped(q, asker.source)
        && (triggering.source == asker.source || (q.attached_to(triggering.source) == Some(asker.source) && is_equipment(q, triggering.source)))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(THIS_ENTERS, search, "When Cloud enters, search your library for an Equipment card, reveal it, put it into your hand, then shuffle.")],
    additional_triggers: &[triggers_again],
    ..oracle::CLOUD_MIDGAR_MERCENARY
};
