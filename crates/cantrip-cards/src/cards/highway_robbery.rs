use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::plot::plot;
use cantrip_oracle as oracle;

/// "You may discard a card or sacrifice a land. If you do, draw two cards." (CR 608.2c-d, 701.9, 701.21): chosen as the spell
/// resolves; the draw happens only if a card was discarded or a land was sacrificed. The options are `labels`.
pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::may_sacrifice_or_discard_to_draw(g, item, CardType::Land, 2)),
    abilities: &[plot(
        mana("{1}{R}"),
        "Plot {1}{R} (You may pay {1}{R} and exile this card from your hand. Cast it as a sorcery on a later turn without paying its mana cost. Plot only as a sorcery.)",
    )],
    labels: &["Sacrifice a land", "Discard a card", "Neither"],
    ..oracle::HIGHWAY_ROBBERY
};
