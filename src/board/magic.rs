use fastrand;

// Magic bitboards: https://www.chessprogramming.org/Magic_Bitboards

fn random_u64() -> u64 {
    fastrand::u64(..)
}

fn random_u64_fewbits() -> u64 {
    random_u64() & random_u64() & random_u64()
}

fn count_1s(mut b: u64) -> u8 {
    let mut r = 0;
    while b > 0 {
        r += 1;
        b &= b - 1
    }
    r
}

const BIT_TABLE: [u8; 64] = [
    63, 30, 3, 32, 25, 41, 22, 33, 15, 50, 42, 13, 11, 53, 19, 34, 61, 29, 2, 51, 21, 43, 45, 10,
    18, 47, 1, 54, 9, 57, 0, 35, 62, 31, 40, 4, 49, 5, 52, 26, 60, 6, 23, 44, 46, 27, 56, 16, 7,
    39, 48, 24, 59, 14, 12, 55, 38, 28, 58, 20, 37, 17, 36, 8,
];

fn pop_1st_bit(bb: &mut u64) -> u8 {
    let b: u64 = *bb ^ (*bb - 1);
    let fold: u64 = (b & 0xFFFFFFFF) ^ (b >> 32);
    *bb &= *bb - 1;
    BIT_TABLE[(((fold * 0x783a9b23) as u32) >> 26) as usize]
}

fn index_to_u64(index: usize, bits: u8, m: &mut u64) -> u64 {
    let mut result: u64 = 0;
    for i in 0..bits {
        let j = pop_1st_bit(m);
        if index & (1 << i) > 0 {
            result |= 1 << j;
        }
    }
    result
}

fn rmask(sq: u64) -> u64 {
    let mut result: u64 = 0;
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut r;
    let mut f;

    r = rank + 1;
    while r <= 6 {
        result |= 1 << (file + r * 8);
        r += 1;
    }

    r = rank - 1;
    while r >= 1 {
        result |= 1 << (file + r * 8);
        r -= 1;
    }

    f = file + 1;
    while f <= 6 {
        result |= 1 << (f + rank * 8);
        f += 1;
    }

    f = file - 1;
    while f >= 1 {
        result |= 1 << (f + rank * 8);
        f -= 1;
    }

    result
}

fn bmask(sq: u64) -> u64 {
    let mut result: u64 = 0;
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut r;
    let mut f;

    r = rank + 1;
    f = file + 1;
    while r <= 6 && f <= 6 {
        result |= 1 << (f + r * 8);
        r += 1;
        f += 1;
    }

    r = rank + 1;
    f = file - 1;
    while r <= 6 && f >= 1 {
        result |= 1 << (f + r * 8);
        r += 1;
        f -= 1;
    }

    r = rank - 1;
    f = file + 1;
    while r >= 1 && f <= 6 {
        result |= 1 << (f + r * 8);
        r -= 1;
        f += 1;
    }

    r = rank - 1;
    f = file - 1;
    while r >= 1 && f >= 1 {
        result |= 1 << (f + r * 8);
        r -= 1;
        f -= 1;
    }

    result
}

fn ratt(sq: u64, block: u64) -> u64 {
    let mut result: u64 = 0;
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut r;
    let mut f;

    r = rank + 1;
    while r <= 7 {
        result |= 1 << (file + r * 8);
        if block & (1 << (file + r * 8)) > 0 {
            break;
        }
        r += 1;
    }

    r = rank - 1;
    while r >= 0 {
        result |= 1 << (file + r * 8);
        if block & (1 << (file + r * 8)) > 0 {
            break;
        }
        r -= 1;
    }

    f = file + 1;
    while f <= 7 {
        result |= 1 << (f + rank * 8);
        if block & (1 << (f + rank * 8)) > 0 {
            break;
        }
        f += 1;
    }

    f = file - 1;
    while f >= 0 {
        result |= 1 << (f + rank * 8);
        if block & (1 << (f + rank * 8)) > 0 {
            break;
        }
        f -= 1;
    }

    result
}

fn batt(sq: u64, block: u64) -> u64 {
    let mut result: u64 = 0;
    let rank = (sq / 8) as i8;
    let file = (sq % 8) as i8;
    let mut r;
    let mut f;

    r = rank + 1;
    f = file + 1;
    while r <= 7 && f <= 7 {
        result |= 1 << (f + r * 8);
        if block & (1 << (f + r * 8)) > 0 {
            break;
        }
        r += 1;
        f += 1;
    }

    r = rank + 1;
    f = file - 1;
    while r <= 7 && f >= 0 {
        result |= 1 << (f + r * 8);
        if block & (1 << (f + r * 8)) > 0 {
            break;
        }
        r += 1;
        f -= 1;
    }

    r = rank - 1;
    f = file + 1;
    while r >= 0 && f <= 7 {
        result |= 1 << (f + r * 8);
        if block & (1 << (f + r * 8)) > 0 {
            break;
        }
        r -= 1;
        f += 1;
    }

    r = rank - 1;
    f = file - 1;
    while r >= 0 && f >= 0 {
        result |= 1 << (f + r * 8);
        if block & (1 << (f + r * 8)) > 0 {
            break;
        }
        r -= 1;
        f -= 1;
    }

    result
}

fn transform(b: u64, magic: u64, bits: u8) -> u64 {
    let (what, _) = b.overflowing_mul(magic);
    what >> (64 - bits)
}

pub fn find_magic(sq: u64, bits: u8, bishop: bool) -> u64 {
    let mut b: [u64; 4096] = [0; 4096];
    let mut a: [u64; 4096] = [0; 4096];

    let mask: u64 = match bishop {
        true => bmask(sq),
        false => rmask(sq),
    };

    let n = count_1s(mask);
    for i in 0..(1 << n) {
        let mut m = mask;
        b[i] = index_to_u64(i, n, &mut m);
        a[i] = match bishop {
            true => batt(sq, b[i]),
            false => ratt(sq, b[i]),
        };
    }

    let mut used: [u64; 4096] = [0; 4096];

    for _ in 0..100000000 {
        let magic = random_u64_fewbits();
        let (what, _) = mask.overflowing_mul(magic);
        if count_1s(what & 0xFF00000000000000) < 6 {
            continue;
        }

        for item in &mut used {
            *item = 0;
        }

        let mut fail = false;

        for i in 0..(1 << n) {
            let j = transform(b[i], magic, bits) as usize;
            if used[j] == 0 {
                used[j] = a[i];
            } else if used[j] != a[i] {
                fail = true;
                break;
            }
        }
        if !fail {
            return magic;
        }
    }

    println!("***FAILED to find magic***");
    0
}

const RBITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 12, 11, 11, 11, 11, 11, 11, 12,
];

const BBITS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 6,
];

pub fn run_find_magic() {
    println!("const RMAGIC: [u64; 64] = [");
    for square in 0..64 {
        let magic = find_magic(square, RBITS[square as usize], false);
        println!("  {},", magic);
    }
    println!("];");

    println!("const BMAGIC: [u64; 64] = [");
    for square in 0..64 {
        let magic = find_magic(square, BBITS[square as usize], true);
        println!("{},", magic);
    }
    println!("];");
}

// cargo run find-magic

pub const RMAGIC: [u64; 64] = [
    180144536750653472,
    306262367384125441,
    144132917868642432,
    144119629074141249,
    36037593179127810,
    2377905001314910728,
    2594117435095385089,
    2341880881498562688,
    1297740381198254112,
    4612249243896545536,
    2613917439951904768,
    140806208356480,
    4613374928418244705,
    577727398296945665,
    282578783568384,
    9225060906046917666,
    2395951835413217408,
    45072281231229058,
    40603865439076376,
    36066180687069192,
    13841814004613513344,
    291046229725938688,
    5188441440116146440,
    4611688218004308100,
    9223512931109978113,
    299559745040158725,
    17594335625344,
    18472113175282176,
    1188954701820330112,
    4400202383872,
    132027294745104,
    9354441654337,
    612630561697038376,
    12700159745816793089,
    4620693356211605760,
    4684447334342332416,
    1729386657146210304,
    562984346718212,
    18032545887160322,
    4906812670029598976,
    4755942218871635968,
    144150441171632128,
    14126103815824474144,
    1688884758052928,
    4620711909514084357,
    182958743519002752,
    146929944419762180,
    10133787581743108,
    2522482534017861888,
    723461198494171648,
    3458905320030470784,
    35253092417792,
    9024825817302272,
    2306407076127585792,
    8802804040704,
    563242021753344,
    306825467158347906,
    4612961529778683905,
    9295500086073100545,
    4503668481392673,
    446420429890916354,
    648799855711617025,
    36046391369402372,
    72060085664289794,
];

pub const BMAGIC: [u64; 64] = [
    5206163935535046721,
    4760588482296020992,
    1157426074901285888,
    153299422666559488,
    2882868979365384226,
    642252297732096,
    2616740934299156745,
    1297055401595896897,
    2314920648155136064,
    162272815742200576,
    9225417454908473344,
    289396081958060036,
    4620714180343758913,
    648519515243815936,
    2201439700996,
    144116305052633216,
    1747537427939461254,
    1157513084761407552,
    2254565906845712,
    1226109535127609602,
    289356422098059264,
    1152956697578326029,
    5778683506478817344,
    569551326577668,
    1130298222850060,
    10376860082545099268,
    324298781393160704,
    577586927155347458,
    9799977929029984256,
    290517498078560516,
    5764748818935580674,
    2325135085357105289,
    2326584862181376,
    144679254739683328,
    4611756593598955584,
    5765171023280472576,
    582094667072540688,
    19176049858445568,
    565157835309184,
    4620836155268563968,
    2307114118340952192,
    565217730758016,
    1531294396685750274,
    2312633739124938752,
    46461101055150593,
    567382368126984,
    2317104516577035356,
    307867567530052,
    282608885858312,
    18579624829255968,
    77126893608370241,
    72097213031972864,
    75195614208,
    54049930107486288,
    1173205299583254536,
    19440636922986497,
    564067190440962,
    2218494545921,
    144115207407936000,
    5188287579088259073,
    585467999894388992,
    9364237717762,
    18023203264007169,
    297272798438129795,
];
