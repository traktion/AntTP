use std::env; 
use std::fs::File; 
use std::io::Write; 
use std::path::Path; 
 
fn main() { 
    let out_dir = env::var("OUT_DIR").unwrap(); 
    let dest_path = Path::new(&out_dir).join("lib.rs"); 
    let mut f = File::create(&dest_path).unwrap(); 
 
/// Unroll the given for loop 
/// 
/// Example: 
/// 
/// ```rust 
/// # #[macro_use] extern crate crunchy; 
/// # fn main() { 
/// let mut x = 0; 
/// unroll! { 
///     for i in 0..10 { 
///         x += i; 
///     } 
/// } 
/// # } 
/// ``` 
#[macro_export] 
macro_rules! unroll { 
    (for $v:ident in 0..$e:expr $c:block) => { 
        { 
            let max = $e; 
            assert!(max <= 128, "Unroll is not designed for large loops"); 
            #[allow(unused_comparisons)] 
            { 
                if max > 0 { let $v = 0; $c } 
                if max > 1 { let $v = 1; $c } 
                if max > 2 { let $v = 2; $c } 
                if max > 3 { let $v = 3; $c } 
                if max > 4 { let $v = 4; $c } 
                if max > 5 { let $v = 5; $c } 
                if max > 6 { let $v = 6; $c } 
                if max > 7 { let $v = 7; $c } 
                if max > 8 { let $v = 8; $c } 
                if max > 9 { let $v = 9; $c } 
                if max > 10 { let $v = 10; $c } 
                if max > 11 { let $v = 11; $c } 
                if max > 12 { let $v = 12; $c } 
                if max > 13 { let $v = 13; $c } 
                if max > 14 { let $v = 14; $c } 
                if max > 15 { let $v = 15; $c } 
                if max > 16 { let $v = 16; $c } 
                if max > 17 { let $v = 17; $c } 
                if max > 18 { let $v = 18; $c } 
                if max > 19 { let $v = 19; $c } 
                if max > 20 { let $v = 20; $c } 
                if max > 21 { let $v = 21; $c } 
                if max > 22 { let $v = 22; $c } 
                if max > 23 { let $v = 23; $c } 
                if max > 24 { let $v = 24; $c } 
                if max > 25 { let $v = 25; $c } 
                if max > 26 { let $v = 26; $c } 
                if max > 27 { let $v = 27; $c } 
                if max > 28 { let $v = 28; $c } 
                if max > 29 { let $v = 29; $c } 
                if max > 30 { let $v = 30; $c } 
                if max > 31 { let $v = 31; $c } 
                if max > 32 { let $v = 32; $c } 
                if max > 33 { let $v = 33; $c } 
                if max > 34 { let $v = 34; $c } 
                if max > 35 { let $v = 35; $c } 
                if max > 36 { let $v = 36; $c } 
                if max > 37 { let $v = 37; $c } 
                if max > 38 { let $v = 38; $c } 
                if max > 39 { let $v = 39; $c } 
                if max > 40 { let $v = 40; $c } 
                if max > 41 { let $v = 41; $c } 
                if max > 42 { let $v = 42; $c } 
                if max > 43 { let $v = 43; $c } 
                if max > 44 { let $v = 44; $c } 
                if max > 45 { let $v = 45; $c } 
                if max > 46 { let $v = 46; $c } 
                if max > 47 { let $v = 47; $c } 
                if max > 48 { let $v = 48; $c } 
                if max > 49 { let $v = 49; $c } 
                if max > 50 { let $v = 50; $c } 
                if max > 51 { let $v = 51; $c } 
                if max > 52 { let $v = 52; $c } 
                if max > 53 { let $v = 53; $c } 
                if max > 54 { let $v = 54; $c } 
                if max > 55 { let $v = 55; $c } 
                if max > 56 { let $v = 56; $c } 
                if max > 57 { let $v = 57; $c } 
                if max > 58 { let $v = 58; $c } 
                if max > 59 { let $v = 59; $c } 
                if max > 60 { let $v = 60; $c } 
                if max > 61 { let $v = 61; $c } 
                if max > 62 { let $v = 62; $c } 
                if max > 63 { let $v = 63; $c } 
                if max > 64 { let $v = 64; $c } 
                if max > 65 { let $v = 65; $c } 
                if max > 66 { let $v = 66; $c } 
                if max > 67 { let $v = 67; $c } 
                if max > 68 { let $v = 68; $c } 
                if max > 69 { let $v = 69; $c } 
                if max > 70 { let $v = 70; $c } 
                if max > 71 { let $v = 71; $c } 
                if max > 72 { let $v = 72; $c } 
                if max > 73 { let $v = 73; $c } 
                if max > 74 { let $v = 74; $c } 
                if max > 75 { let $v = 75; $c } 
                if max > 76 { let $v = 76; $c } 
                if max > 77 { let $v = 77; $c } 
                if max > 78 { let $v = 78; $c } 
                if max > 79 { let $v = 79; $c } 
                if max > 80 { let $v = 80; $c } 
                if max > 81 { let $v = 81; $c } 
                if max > 82 { let $v = 82; $c } 
                if max > 83 { let $v = 83; $c } 
                if max > 84 { let $v = 84; $c } 
                if max > 85 { let $v = 85; $c } 
                if max > 86 { let $v = 86; $c } 
                if max > 87 { let $v = 87; $c } 
                if max > 88 { let $v = 88; $c } 
                if max > 89 { let $v = 89; $c } 
                if max > 90 { let $v = 90; $c } 
                if max > 91 { let $v = 91; $c } 
                if max > 92 { let $v = 92; $c } 
                if max > 93 { let $v = 93; $c } 
                if max > 94 { let $v = 94; $c } 
                if max > 95 { let $v = 95; $c } 
                if max > 96 { let $v = 96; $c } 
                if max > 97 { let $v = 97; $c } 
                if max > 98 { let $v = 98; $c } 
                if max > 99 { let $v = 99; $c } 
                if max > 100 { let $v = 100; $c } 
                if max > 101 { let $v = 101; $c } 
                if max > 102 { let $v = 102; $c } 
                if max > 103 { let $v = 103; $c } 
                if max > 104 { let $v = 104; $c } 
                if max > 105 { let $v = 105; $c } 
                if max > 106 { let $v = 106; $c } 
                if max > 107 { let $v = 107; $c } 
                if max > 108 { let $v = 108; $c } 
                if max > 109 { let $v = 109; $c } 
                if max > 110 { let $v = 110; $c } 
                if max > 111 { let $v = 111; $c } 
                if max > 112 { let $v = 112; $c } 
                if max > 113 { let $v = 113; $c } 
                if max > 114 { let $v = 114; $c } 
                if max > 115 { let $v = 115; $c } 
                if max > 116 { let $v = 116; $c } 
                if max > 117 { let $v = 117; $c } 
                if max > 118 { let $v = 118; $c } 
                if max > 119 { let $v = 119; $c } 
                if max > 120 { let $v = 120; $c } 
                if max > 121 { let $v = 121; $c } 
                if max > 122 { let $v = 122; $c } 
                if max > 123 { let $v = 123; $c } 
                if max > 124 { let $v = 124; $c } 
                if max > 125 { let $v = 125; $c } 
                if max > 126 { let $v = 126; $c } 
                if max > 127 { let $v = 127; $c } 
            } 
        } 
    } 
} 
 
    f.write_all(crunchy_lib.as_bytes()).unwrap(); 
} 
