//! Variables named like their struct type. GLSL scopes a variable's name over the type's
//! (`Face face;` is common, so is `Face Face = getFace(); Face.size`); naga's frontend keeps
//! reading the name as the type after the declaration ("Expected LeftParen, found Dot"). Each such
//! variable gets a name of its own, `<name>_hzv`, at every place the name is used as a value;
//! places that use it as a type (before a name, a constructor's parenthesis, an array type's
//! brackets before a name) and member names (`a.Face`) stay as they are.

use std::collections::BTreeSet;

use super::scan::significant;
use crate::macros::lex;

fn is_name(t: &str) -> bool {
    t.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
}

/// `code` with the shadowing variables renamed, or `None` when no variable shadows a struct.
pub(super) fn rename(code: &str) -> Option<String> {
    let tokens = lex(code);
    // Significant tokens with their place among all tokens.
    let sig: Vec<usize> = (0..tokens.len())
        .filter(|&i| significant(tokens[i]))
        .collect();
    let mut structs: BTreeSet<&str> = BTreeSet::new();
    for w in sig.windows(2) {
        if tokens[w[0]] == "struct" && is_name(tokens[w[1]]) {
            structs.insert(tokens[w[1]]);
        }
    }
    if structs.is_empty() {
        return None;
    }
    // `S S` followed by what ends a declarator.
    let mut shadowed: BTreeSet<&str> = BTreeSet::new();
    for w in sig.windows(3) {
        let (a, b, c) = (tokens[w[0]], tokens[w[1]], tokens[w[2]]);
        if a == b && structs.contains(a) && matches!(c, "=" | ";" | "," | ")" | "[") {
            shadowed.insert(a);
        }
    }
    if shadowed.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(code.len() + 64);
    let mut position: usize = 0; // index into `sig` of the token being written
    for t in &tokens {
        if !significant(t) {
            out.push_str(t);
            continue;
        }
        let k = position;
        position += 1;
        if !shadowed.contains(t) {
            out.push_str(t);
            continue;
        }
        let previous = k.checked_sub(1).map(|p| tokens[sig[p]]);
        let mut next = k + 1;
        // An array type's brackets (`S[2] a`), skipped to see what follows them.
        if sig.get(next).is_some_and(|&n| tokens[n] == "[") {
            let mut depth = 0;
            while let Some(&n) = sig.get(next) {
                match tokens[n] {
                    "[" => depth += 1,
                    "]" => depth -= 1,
                    _ => {}
                }
                next += 1;
                if depth == 0 {
                    break;
                }
            }
        }
        let following = sig.get(next).map(|&n| tokens[n]);
        // A name before a name is a type; so is one before a parenthesis (a constructor).
        let as_type = following.is_some_and(is_name) || following == Some("(");
        let member = previous == Some(".");
        let declaration = previous == Some("struct");
        out.push_str(t);
        if !as_type && !member && !declaration {
            out.push_str("_hzv");
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::rename;

    #[test]
    fn a_variable_named_like_its_struct_is_renamed_where_it_is_a_value() {
        let code = "struct Face { vec4 bounds; };\n\
                    vec4 f(Face a) {\n    Face Face = make(a);\n    if (Face.bounds.z < 1.0) return Face(a.bounds);\n    \
                    Face list[2];\n    return take(Face, list[0].bounds) + a.Face;\n}\n";
        assert_eq!(
            rename(code).as_deref(),
            Some(
                "struct Face { vec4 bounds; };\n\
                 vec4 f(Face a) {\n    Face Face_hzv = make(a);\n    if (Face_hzv.bounds.z < 1.0) return Face(a.bounds);\n    \
                 Face list[2];\n    return take(Face_hzv, list[0].bounds) + a.Face;\n}\n"
            )
        );
        assert_eq!(rename("struct A { int a; };\nA b = A(1);\n"), None);
    }
}
