use cantrip_api::*;
use cantrip_oracle as oracle;

/// "Black spells you cast cost {B} more to cast" (CR 601.2f): a cost increase of one black mana, asked of the spell's own
/// characteristics as it is cast; only its controller's spells. "Black" is a color word of Derelor's text (CR 612.2); the {B}
/// is a mana symbol, not a color word, so changing the color word leaves it {B}.
fn black_spells_cost_more(q: &dyn Query, source: ObjectId, player: PlayerId, costed: Costed, _: &[Target]) -> CostDelta {
    let Costed::Spell(spell) = costed else { return CostDelta::NONE };
    if player != q.controller(source) {
        return CostDelta::NONE;
    }
    if q.chars(spell).colors.has(q.words(source).color(Color::Black)) { CostDelta::more_mana(mana("{B}")) } else { CostDelta::NONE }
}

pub static CARD: CardDef = CardDef { cost_changes: &[black_spells_cost_more], ..oracle::DERELOR };
