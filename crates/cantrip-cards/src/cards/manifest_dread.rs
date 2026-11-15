use cantrip_api::*;
use cantrip_mechanics::manifest_dread::controller_manifests_dread;
use cantrip_oracle as oracle;

pub static CARD: CardDef = CardDef { spell: spell(&[], controller_manifests_dread), ..oracle::MANIFEST_DREAD };
