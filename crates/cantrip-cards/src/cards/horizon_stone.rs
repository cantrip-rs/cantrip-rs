use cantrip_api::*;
use cantrip_oracle as oracle;

/// "If you would lose unspent mana, that mana becomes colorless instead" (CR 106.4, 614.1a): the controller's mana only.
fn your_unspent_mana_becomes_colorless(_: &dyn Query, asker: &Asker, player: PlayerId) -> Option<Mana> {
    (player == asker.controller).then_some(Mana::C)
}

pub static CARD: CardDef = CardDef { mana_lost_becomes: &[your_unspent_mana_becomes_colorless], ..oracle::HORIZON_STONE };
