#[path = "support/discard_library.rs"]
mod discard_library;
use discard_library::play;

#[test]
fn land_discarded_to_library_remains_revealed_for_aclazotz() {
    play("wand-of-ith-revealed-discard", r#"
[Bot1 Deck]
Black Lotus x4
Mox Pearl
Aclazotz, Deepest Betrayal
Wand of Ith
Island x20
[Bot2 Deck]
Probe Leng
Island x20
[Dice]
1
[Actions]
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Black Lotus.Cast
Bot1.Hand.Mox Pearl.Cast
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Battlefield.Black Lotus.Activate(Color: Black)
Bot1.Hand.Aclazotz, Deepest Betrayal.Cast
Bot1.Pass
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Hand.Wand of Ith.Cast
Bot1.Pass
[T2 Bot2.Main1]
Bot2.Hand.Probe Leng.Cast
[T3 Bot1.Main1]
Bot1.Battlefield.Black Lotus.Activate(Color: White)
Bot1.Battlefield.Wand of Ith.Activate(Bot2)
[T3 Bot1.End]
[Expect]
Bot2.Graveyard.Count = 0
Bot1.Battlefield.Bat Token.Token = true
"#);
}
