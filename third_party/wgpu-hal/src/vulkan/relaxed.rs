//! Starling: half precision for a fragment shader, which a phone's GPU runs cheaper. A shader pack's
//! GLSL says nothing about precision, so this works out what can do without: the floats computed
//! for colours. What decides where a texture is read, what is compared, what becomes an integer and
//! what comes from the pixel's position on the screen stays at full precision, with all it is made
//! of.

use std::{vec, vec::Vec};

const OP_EXT_INST: u32 = 12;
const OP_TYPE_FLOAT: u32 = 22;
const OP_TYPE_VECTOR: u32 = 23;
const OP_TYPE_MATRIX: u32 = 24;
const OP_TYPE_POINTER: u32 = 32;
const OP_FUNCTION: u32 = 54;
const OP_FUNCTION_PARAMETER: u32 = 55;
const OP_FUNCTION_CALL: u32 = 57;
const OP_VARIABLE: u32 = 59;
const OP_LOAD: u32 = 61;
const OP_STORE: u32 = 62;
const OP_ACCESS_CHAIN: u32 = 65;
const OP_IN_BOUNDS_ACCESS_CHAIN: u32 = 66;
const OP_DECORATE: u32 = 71;
const OP_VECTOR_EXTRACT_DYNAMIC: u32 = 77;
const OP_VECTOR_INSERT_DYNAMIC: u32 = 78;
const OP_VECTOR_SHUFFLE: u32 = 79;
const OP_COMPOSITE_CONSTRUCT: u32 = 80;
const OP_COMPOSITE_EXTRACT: u32 = 81;
const OP_COMPOSITE_INSERT: u32 = 82;
const OP_COPY_OBJECT: u32 = 83;
const OP_TRANSPOSE: u32 = 84;
const OP_CONVERT_F_TO_U: u32 = 109;
const OP_CONVERT_F_TO_S: u32 = 110;
const OP_F_CONVERT: u32 = 115;
const OP_BITCAST: u32 = 124;
const OP_F_NEGATE: u32 = 127;
const OP_DOT: u32 = 148;
const OP_IS_NAN: u32 = 156;
const OP_IS_INF: u32 = 157;
const OP_SELECT: u32 = 169;
const OP_DPDX: u32 = 207;
const OP_FWIDTH_COARSE: u32 = 215;
const OP_PHI: u32 = 245;
const OP_RETURN_VALUE: u32 = 254;

const DECORATION_RELAXED_PRECISION: u32 = 0;
const DECORATION_BUILT_IN: u32 = 11;
const BUILT_IN_FRAG_COORD: u32 = 15;
const STORAGE_FUNCTION: u32 = 7;

/// The module's words, or a copy of them with `RelaxedPrecision` on what can do without.
pub fn relax(spv: &[u32]) -> Vec<u32> {
    let Some(&bound) = spv.get(3) else {
        return spv.to_vec();
    };
    let bound = bound as usize;
    let mut float = vec![false; bound];
    let mut float_pointer = vec![false; bound];
    // What a value is made of, and what is made of it.
    let mut made_of = vec![Vec::new(); bound];
    let mut goes_into = vec![Vec::new(); bound];
    let mut full = Vec::new();
    let mut screen = Vec::new();
    let mut calls = Vec::new();
    let mut params: Vec<(u32, Vec<u32>)> = Vec::new();
    let mut function = None;
    let mut first_type = None;

    let mut link = |result: u32, operand: u32| {
        if (result as usize) < bound && (operand as usize) < bound {
            made_of[result as usize].push(operand);
            goes_into[operand as usize].push(result);
        }
    };
    for (at, op, w) in instructions(spv) {
        let id = |i: usize| w.get(i).copied().unwrap_or(u32::MAX);
        if (19..=50).contains(&op) && first_type.is_none() {
            first_type = Some(at);
        }
        match op {
            OP_TYPE_FLOAT if id(2) == 32 => float[id(1) as usize] = true,
            OP_TYPE_VECTOR | OP_TYPE_MATRIX if is(&float, id(2)) => float[id(1) as usize] = true,
            OP_TYPE_POINTER if is(&float, id(3)) => float_pointer[id(1) as usize] = true,
            OP_DECORATE if id(2) == DECORATION_BUILT_IN => {
                full.push(id(1));
                if id(3) == BUILT_IN_FRAG_COORD {
                    screen.push(id(1));
                }
            }
            OP_FUNCTION => {
                function = Some(id(2));
                params.push((id(2), Vec::new()));
            }
            OP_FUNCTION_PARAMETER => {
                if let Some(last) = params.last_mut() {
                    last.1.push(id(2));
                }
            }
            OP_RETURN_VALUE => {
                if let Some(f) = function {
                    link(f, id(1));
                }
            }
            OP_FUNCTION_CALL => {
                link(id(2), id(3));
                for arg in w.iter().skip(4) {
                    link(id(2), *arg);
                }
                calls.push((id(3), w[4.min(w.len())..].to_vec()));
            }
            OP_LOAD | OP_COPY_OBJECT | OP_TRANSPOSE | OP_F_CONVERT | OP_BITCAST => {
                link(id(2), id(3));
            }
            OP_STORE => link(id(1), id(2)),
            OP_ACCESS_CHAIN | OP_IN_BOUNDS_ACCESS_CHAIN => {
                link(id(2), id(3));
                // A store through it is a store to what it points in.
                link(id(3), id(2));
                for index in w.iter().skip(4) {
                    link(id(2), *index);
                }
            }
            OP_EXT_INST => {
                for arg in w.iter().skip(5) {
                    link(id(2), *arg);
                }
            }
            OP_COMPOSITE_EXTRACT => link(id(2), id(3)),
            OP_COMPOSITE_INSERT | OP_VECTOR_SHUFFLE => {
                link(id(2), id(3));
                link(id(2), id(4));
            }
            OP_PHI => {
                for value in w.iter().skip(3).step_by(2) {
                    link(id(2), *value);
                }
            }
            OP_COMPOSITE_CONSTRUCT
            | OP_VECTOR_EXTRACT_DYNAMIC
            | OP_VECTOR_INSERT_DYNAMIC
            | OP_SELECT
            | OP_F_NEGATE..=OP_DOT
            | OP_DPDX..=OP_FWIDTH_COARSE => {
                for operand in w.iter().skip(3) {
                    link(id(2), *operand);
                }
            }
            // Sampling, fetching, gathering and reading: the coordinates, and the rest with them.
            87..=98 => full.extend(w.iter().skip(4).copied()),
            // Comparing, and becoming an integer.
            180..=191 | OP_IS_NAN | OP_IS_INF | OP_CONVERT_F_TO_U | OP_CONVERT_F_TO_S => {
                full.extend(w.iter().skip(3).copied());
            }
            _ => {}
        }
    }
    for (callee, args) in calls {
        let Some((_, list)) = params.iter().find(|(f, _)| *f == callee) else {
            continue;
        };
        for (param, arg) in list.iter().zip(args) {
            link(*param, arg);
        }
    }

    let keep_full = reached(full, &made_of, bound);
    let keep_screen = reached(screen, &goes_into, bound);
    let relaxed = |result: u32| {
        let r = result as usize;
        r < bound && !keep_full[r] && !keep_screen[r]
    };
    let mut decorate = Vec::new();
    for (_, op, w) in instructions(spv) {
        let id = |i: usize| w.get(i).copied().unwrap_or(u32::MAX);
        let computes = matches!(
            op,
            OP_EXT_INST
                | OP_LOAD
                | OP_VECTOR_SHUFFLE
                | OP_COMPOSITE_CONSTRUCT
                | OP_COMPOSITE_EXTRACT
                | OP_SELECT
                | OP_PHI
                | OP_F_NEGATE
                | 129
                | 131
                | 133
                | 136
                | 140..=146 | OP_DOT
        );
        if computes && is(&float, id(1)) && relaxed(id(2)) {
            decorate.push(id(2));
        }
        if op == OP_VARIABLE
            && id(3) == STORAGE_FUNCTION
            && is(&float_pointer, id(1))
            && relaxed(id(2))
        {
            decorate.push(id(2));
        }
    }
    let at = first_type.unwrap_or(5);
    let mut out = Vec::with_capacity(spv.len() + decorate.len() * 3);
    out.extend_from_slice(&spv[..at]);
    for target in decorate {
        out.extend([3 << 16 | OP_DECORATE, target, DECORATION_RELAXED_PRECISION]);
    }
    out.extend_from_slice(&spv[at..]);
    out
}

fn is(set: &[bool], id: u32) -> bool {
    set.get(id as usize).copied().unwrap_or(false)
}

/// Each instruction after the header: where it starts, its opcode and its words.
fn instructions(spv: &[u32]) -> impl Iterator<Item = (usize, u32, &[u32])> {
    let mut at = 5;
    std::iter::from_fn(move || {
        let head = *spv.get(at)?;
        let count = (head >> 16) as usize;
        let words = spv.get(at..at + count).filter(|_| count > 0)?;
        let start = at;
        at += count;
        Some((start, head & 0xffff, words))
    })
}

/// All that can be reached from `from`, by `edges`.
fn reached(from: Vec<u32>, edges: &[Vec<u32>], bound: usize) -> Vec<bool> {
    let mut seen = vec![false; bound];
    let mut stack = from;
    while let Some(id) = stack.pop() {
        let i = id as usize;
        if i < bound && !seen[i] {
            seen[i] = true;
            stack.extend(edges[i].iter().copied());
        }
    }
    seen
}
