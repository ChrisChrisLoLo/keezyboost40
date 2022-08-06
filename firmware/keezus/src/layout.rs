
use keyberon::action::{k, Action, Action::*};
use keyberon::key_code::KeyCode::*;

use crate::{NUM_COLS, NUM_ROWS, NUM_LAYERS};
#[allow(unused_macros)]

// Shift + KeyCode
macro_rules! s {
    ($k:ident) => {
        m(&[LShift, $k])
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CustomActions {
    Bootloader,
}

#[allow(dead_code)]
const BOOTLOADER: Action<CustomActions> = Action::Custom(CustomActions::Bootloader);

#[rustfmt::skip]
pub static LAYERS: keyberon::layout::Layers<NUM_COLS, NUM_ROWS, NUM_LAYERS, CustomActions> = [
    /* QWERTY */
    /* 
        All Trans keys are placeholders to even out the layout
        All k(No) keys are functional
    */
    [
        [k(Q),    k(W),    k(E),   k(R),   k(T),     k(Y),   k(U),   k(I),     k(O),     k(P)],
        [k(A),    k(S),    k(D),   k(F),   k(G),     k(H),   k(J),   k(K),     k(L),     k(SColon)],
        [k(Z),    k(X),    k(C),   k(V),   k(B),     k(N),   k(M),   k(Comma), k(Dot),   k(Slash)],
        [ k(LGui), k(LAlt), Trans,  Trans,  k(Space), Trans,  Trans,  k(RAlt),  k(RCtrl), k(No),],
    ] 
];