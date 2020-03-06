fn try_identity(x: Result<u32, i32>) -> Result<u32, i32> {
    let y = x?;
    Ok(y)
}

fn main() {
    let _ = try_identity(Ok(0));
}

// END RUST SOURCE
// START rustc.try_identity.SimplifyArmIdentity.before.mir
// fn try_identity(_1: std::result::Result<u32, i32>) -> std::result::Result<u32, i32> {
//     debug x => _1;
//     let mut _0: std::result::Result<u32, i32>;
//     let _2: u32;
//     let mut _3: std::result::Result<u32, i32>;
//     let mut _4: std::result::Result<u32, i32>;
//     let mut _5: isize;
//     let _6: i32;
//     let mut _7: !;
//     let mut _8: i32;
//     let mut _9: i32;
//     let _10: u32;
//     let mut _11: u32;
//     scope 1 {
//         debug y => _2;
//     }
//     scope 2 {
//         debug err => _6;
//         scope 3 {
//             scope 7 {
//                 debug t => _9;
//             }
//             scope 8 {
//                 debug v => _8;
//                 let mut _12: i32;
//             }
//         }
//     }
//     scope 4 {
//         debug val => _10;
//         scope 5 {
//         }
//     }
//     scope 6 {
//         debug self => _4;
//     }
//     bb0: {
//         StorageLive(_2);
//         StorageLive(_3);
//         StorageLive(_4);
//         _4 = _1;
//         _3 = move _4;
//         StorageDead(_4);
//         _5 = discriminant(_3);
//         switchInt(move _5) -> [0isize: bb1, otherwise: bb2];
//     }
//     bb1: {
//         StorageLive(_10);
//         _10 = ((_3 as Ok).0: u32);
//         _2 = _10;
//         StorageDead(_10);
//         StorageDead(_3);
//         StorageLive(_11);
//         _11 = _2;
//         ((_0 as Ok).0: u32) = move _11;
//         discriminant(_0) = 0;
//         StorageDead(_11);
//         StorageDead(_2);
//         goto -> bb3;
//     }
//     bb2: {
//         StorageLive(_6);
//         _6 = ((_3 as Err).0: i32);
//         StorageLive(_8);
//         StorageLive(_9);
//         _9 = _6;
//         _8 = move _9;
//         StorageDead(_9);
//         StorageLive(_12);
//         _12 = move _8;
//         ((_0 as Err).0: i32) = move _12;
//         discriminant(_0) = 1;
//         StorageDead(_12);
//         StorageDead(_8);
//         StorageDead(_6);
//         StorageDead(_3);
//         StorageDead(_2);
//         goto -> bb3;
//     }
//     bb3: {
//         return;
//     }
// }
// END rustc.try_identity.SimplifyArmIdentity.before.mir

// START rustc.try_identity.SimplifyArmIdentity.after.mir
// fn try_identity(_1: std::result::Result<u32, i32>) -> std::result::Result<u32, i32> {
//     debug x => _1;
//     let mut _0: std::result::Result<u32, i32>;
//     let _2: u32;
//     let mut _3: std::result::Result<u32, i32>;
//     let mut _4: std::result::Result<u32, i32>;
//     let mut _5: isize;
//     let _6: i32;
//     let mut _7: !;
//     let mut _8: i32;
//     let mut _9: i32;
//     let _10: u32;
//     let mut _11: u32;
//     scope 1 {
//         debug y => _2;
//     }
//     scope 2 {
//         debug err => _6;
//         scope 3 {
//             scope 7 {
//                 debug t => _9;
//             }
//             scope 8 {
//                 debug v => _8;
//                 let mut _12: i32;
//             }
//         }
//     }
//     scope 4 {
//         debug val => _10;
//         scope 5 {
//         }
//     }
//     scope 6 {
//         debug self => _4;
//     }
//     bb0: {
//         StorageLive(_2);
//         StorageLive(_3);
//         StorageLive(_4);
//         _4 = _1;
//         _3 = move _4;
//         StorageDead(_4);
//         _5 = discriminant(_3);
//         switchInt(move _5) -> [0isize: bb1, otherwise: bb2];
//     }
//     bb1: {
//         _0 = move _3;
//         StorageDead(_3);
//         StorageDead(_2);
//         goto -> bb3;
//     }
//     bb2: {
//         _0 = move _3;
//         StorageDead(_3);
//         StorageDead(_2);
//         goto -> bb3;
//     }
//     bb3: {
//         return;
//     }
// }
// END rustc.try_identity.SimplifyArmIdentity.after.mir

// START rustc.try_identity.SimplifyBranchSame.after.mir
// fn try_identity(_1: std::result::Result<u32, i32>) -> std::result::Result<u32, i32> {
//     debug x => _1;
//     let mut _0: std::result::Result<u32, i32>;
//     let _2: u32;
//     let mut _3: std::result::Result<u32, i32>;
//     let mut _4: std::result::Result<u32, i32>;
//     let mut _5: isize;
//     let _6: i32;
//     let mut _7: !;
//     let mut _8: i32;
//     let mut _9: i32;
//     let _10: u32;
//     let mut _11: u32;
//     scope 1 {
//         debug y => _2;
//     }
//     scope 2 {
//         debug err => _6;
//         scope 3 {
//             scope 7 {
//                 debug t => _9;
//             }
//             scope 8 {
//                 debug v => _8;
//                 let mut _12: i32;
//             }
//         }
//     }
//     scope 4 {
//         debug val => _10;
//         scope 5 {
//         }
//     }
//     scope 6 {
//         debug self => _4;
//     }
//     bb0: {
//         StorageLive(_2);
//         StorageLive(_3);
//         StorageLive(_4);
//         _4 = _1;
//         _3 = move _4;
//         StorageDead(_4);
//         _5 = discriminant(_3);
//         goto -> bb1;
//     }
//     bb1: {
//         _0 = move _3;
//         StorageDead(_3);
//         StorageDead(_2);
//         goto -> bb2;
//     }
//     bb2: {
//         return;
//     }
// }
// END rustc.try_identity.SimplifyBranchSame.after.mir

// START rustc.try_identity.SimplifyLocals.after.mir
// fn try_identity(_1: std::result::Result<u32, i32>) -> std::result::Result<u32, i32> {
//     debug x => _1;
//     let mut _0: std::result::Result<u32, i32>;
//     let _2: u32;
//     let mut _3: isize;
//     let _4: i32;
//     let mut _5: i32;
//     let mut _6: i32;
//     let _7: u32;
//     scope 1 {
//         debug y => _2;
//     }
//     scope 2 {
//         debug err => _4;
//         scope 3 {
//             scope 7 {
//                 debug t => _6;
//             }
//             scope 8 {
//                 debug v => _5;
//             }
//         }
//     }
//     scope 4 {
//         debug val => _7;
//         scope 5 {
//         }
//     }
//     scope 6 {
//         debug self => _1;
//     }
//     bb0: {
//         StorageLive(_2);
//         _3 = discriminant(_1);
//         _0 = move _1;
//         StorageDead(_2);
//         return;
//     }
// }
// END rustc.try_identity.SimplifyLocals.after.mir
