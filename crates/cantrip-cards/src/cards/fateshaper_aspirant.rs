use cantrip_api::*;
use cantrip_mechanics::{targets,triggers,effects};
use cantrip_oracle as oracle;
/// Legendary card in your graveyard; card types are unrestricted (CR 115.1).
fn legendary(q:&dyn Query, me:&Asker,t:Target)->bool {
 (targets::CARD_IN_YOUR_GRAVEYARD.filter)(q,me,t) && matches!(t,Target::Object(id) if q.chars(id).supertypes.legendary)
}
static MODAL: ModalDef = modes(&[
 mode(effects::return_first_target_to_hand,"Return target legendary card from your graveyard to your hand.").targets(&[target(legendary)]),
 mode(effects::plus_one_counters_on_first_target_that_gains_until_end_of_turn::<1, { Keywords::of(&[Keyword::Vigilance, Keyword::Indestructible]).0 }>,"Put a +1/+1 counter on target creature. It gains vigilance and indestructible until end of turn.").targets(&[targets::CREATURE]),
]);
pub static CARD:CardDef=CardDef{abilities:&[modal_triggered(triggers::THIS_ENTERS,&MODAL,"When this creature enters, choose one — Return target legendary card from your graveyard to your hand; or put a +1/+1 counter on target creature. It gains vigilance and indestructible until end of turn.")],..oracle::FATESHAPER_ASPIRANT};
