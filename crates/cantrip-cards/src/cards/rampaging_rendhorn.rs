use cantrip_api::*;
use cantrip_mechanics::riot;
use cantrip_oracle as oracle;

/// "Riot" (CR 702.136a): it enters with your choice of a +1/+1 counter or haste.
fn enters(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    riot::enters(g, entering, &CARD)
}

pub static CARD: CardDef = CardDef {
    enters: Some(enters),
    labels: riot::LABELS,
    statics: &[riot::HASTE_STATIC],
    ..oracle::RAMPAGING_RENDHORN
};
