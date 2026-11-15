use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Draw X cards." (CR 107.3, 121.1): X is the value announced as the spell was cast (CR 107.3f); a library asked for more
/// cards than it has is a loss at the next state-based check (CR 704.5b).
pub static CARD: CardDef = CardDef {
    spell: spell(&[], |g, item| effects::draw(g, item, item.x)),
    ..oracle::MIND_SPRING
};
